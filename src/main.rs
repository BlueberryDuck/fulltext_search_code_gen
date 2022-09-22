use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::fs::{read_to_string, File};
use std::io::Write;
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
    let generator = code_gen::generator::generate(ast.unwrap());
    write!(File::create(path)?, "{}", generator.unwrap())
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
    println!("{:?}", run_code_gen(data.search.clone(), PATH_SQL));
    execute_sql(PATH_SQL, PATH_RESULTS);
    let results_vec = read_results(PATH_RESULTS).unwrap();
    let mut results: Vec<Result> = Vec::new();
    for result in results_vec {
        let link = result.0.clone().replace(" ", "_");
        results.push(Result {
            title: result.0,
            rank: result.1,
            link,
        })
    }

    let mut page_data = Context::new();
    page_data.insert("title", "Results");
    page_data.insert("search", &data.search);
    page_data.insert("results", &results);

    let rendered = tera.render("result.html", &page_data).unwrap();
    HttpResponse::Ok().body(rendered)
}
