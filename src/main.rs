use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::fs::{read_to_string, File};
use std::io::{Error, ErrorKind, Write};
use std::process::Command;
use tera::{Context, Tera};

mod code_gen;

// Path Variables
const PATH_SQL: &str = "files\\fulltext.sql";
const PATH_RESULTS: &str = "files\\results.txt";

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| {
        let tera = Tera::new("templates/**/*").unwrap();
        App::new()
            .data(tera)
            .route("/", web::get().to(search))
            .route("/", web::post().to(result))
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}

fn run_code_gen(search: String, path: &str) -> std::io::Result<()> {
    let tokens = code_gen::lexer::lex(search.as_str());
    let ast = code_gen::parser::parse(tokens);
    match ast {
        Ok(ast) => {
            let generator = code_gen::generator::generate(ast);
            match generator {
                Ok(generator) => write!(File::create(path)?, "{}", generator),
                Err(gen_err) => Err(Error::new(ErrorKind::InvalidData, format!("{:?}", gen_err))),
            }
        }
        Err(parse_err) => Err(Error::new(
            ErrorKind::InvalidInput,
            format!("{:?}", parse_err),
        )),
    }
}

fn execute_sql(sql_path: &str, results_path: &str) -> String {
    let command = Command::new("cmd")
        .args(&[
            "/C",
            "sqlcmd",
            "-S",
            "DESKTOP-JKNEH40\\SQLEXPRESS",
            "-i",
            sql_path,
            "-o",
            results_path,
        ])
        .output()
        .expect("failed to execute operation");
    format!("{}", command.status)
}

fn read_results(path: &str) -> Option<Vec<(String, u64)>> {
    let contents = read_to_string(path).unwrap();
    let mut vec: Vec<&str> = contents.split("\n").collect();

    if vec.len() < 6 {
        return None;
    }

    // Remove metadata rows (first three or four and last three rows)
    while !vec[0].starts_with("---") {
        vec.remove(0);
    }
    vec.remove(0);
    vec.remove(vec.len() - 1);
    vec.remove(vec.len() - 1);
    vec.remove(vec.len() - 1);

    let mut results: Vec<(String, u64)> = Vec::new();
    for row in vec {
        let row = row.replace("\r", "");
        let re = Regex::new(r"\s+").unwrap();
        let row = re.replace_all(&row, " ").to_string();

        let mut words: Vec<&str> = row.split(" ").collect();
        let rank = words[words.len() - 1].parse::<u64>().unwrap();
        words.remove(words.len() - 1);
        let title = words.join(" ");

        results.push((title, rank));
    }
    Some(results)
}

// Website stuff
#[derive(Deserialize)]
struct Search {
    search: String,
}
#[derive(Serialize)]
struct Result {
    title: String,
    rank: u64,
    link: String,
}

async fn search(tera: web::Data<Tera>) -> impl Responder {
    let mut data = Context::new();
    data.insert("title", "Search field");

    let rendered = tera.render("search.html", &data).unwrap();
    HttpResponse::Ok().body(rendered)
}

async fn result(tera: web::Data<Tera>, data: web::Form<Search>) -> impl Responder {
    let mut page_data = Context::new();
    let mut results: Vec<Result> = Vec::new();
    match run_code_gen(data.search.clone(), PATH_SQL) {
        Ok(_) => {
            execute_sql(PATH_SQL, PATH_RESULTS);
            let results_vec = read_results(PATH_RESULTS);
            match results_vec {
                Some(results_vec) => {
                    for result in results_vec {
                        results.push(Result {
                            title: result.0.clone(),
                            rank: result.1,
                            link: result.0.replace(" ", "_"),
                        })
                    }
                    page_data.insert("title", "Results");
                    page_data.insert("search", &data.search);
                }
                None => {
                    page_data.insert("title", "Error");
                    page_data.insert(
                        "search",
                        &format!("{} results cannot be read", &data.search),
                    );
                }
            }
        }
        Err(error) => {
            page_data.insert("title", "Error");
            page_data.insert(
                "search",
                &format!("{} threw an error: {}", &data.search, &error.to_string()),
            );
        }
    }
    page_data.insert("results", &results);
    let rendered = tera.render("result.html", &page_data).unwrap();
    HttpResponse::Ok().body(rendered)
}
