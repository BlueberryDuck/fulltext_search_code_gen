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

// Main function to start website on localhost:8080
// Run using 'cargo watch -x run'
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

// Code generator to translate an input to SQL
// Input: search string and path to write result to
// Output: SQL statement written to a file
fn run_code_gen(search: String, path: &str) -> std::io::Result<()> {
    // Transform string to list of tokens
    let tokens = code_gen::lexer::lex(search.as_str());
    // Parse tokens to an abstract syntax tree (ast)
    let ast = code_gen::parser::parse(tokens);
    match ast {
        // If parser returns no error, start code generation
        Ok(ast) => {
            let generator = code_gen::generator::generate(ast);
            // If generator returns no error, write SQL statement to file, otherwise throw an error
            match generator {
                Ok(generator) => write!(File::create(path)?, "{}", generator),
                Err(gen_err) => Err(Error::new(ErrorKind::InvalidData, format!("{:?}", gen_err))),
            }
        }
        // If parser returns error, throw an error aswell
        Err(parse_err) => Err(Error::new(
            ErrorKind::InvalidInput,
            format!("{:?}", parse_err),
        )),
    }
}

// Runs a command to execute an sql statement to a local MSSQL Server
// Input: paths to the input file and where to write the result
// Output: txt file interpretation of the MSSQL Server result
fn execute_sql(sql_path: &str, results_path: &str) {
    Command::new("cmd")
        .args(&[
            "/C",
            "sqlcmd",
            "-S",
            "DESKTOP-JKNEH40\\SQLEXPRESS", //Local server name
            "-i",
            sql_path,
            "-o",
            results_path,
        ])
        .output()
        .expect("failed to execute operation");
}

// Reads the txt file result and extracts the actual results
// Input: path to the txt file
// Output: vec of titles and their search rank
fn read_results(path: &str) -> Option<Vec<(String, u64)>> {
    let contents = read_to_string(path).unwrap();
    let mut contents_vec: Vec<&str> = contents.split("\n").collect();
    // In case of error message, break
    if contents_vec.len() < 6 {
        return None;
    }
    // Remove metadata rows
    // First 3-4 rows and last three rows
    while !contents_vec[0].starts_with("---") {
        contents_vec.remove(0);
    }
    contents_vec.remove(0);
    contents_vec.remove(contents_vec.len() - 1);
    contents_vec.remove(contents_vec.len() - 1);
    contents_vec.remove(contents_vec.len() - 1);
    // Go through each row and extract the titles and their ranks
    let mut results: Vec<(String, u64)> = Vec::new();
    for row in contents_vec {
        // Remove unnecessary whitespaces
        let row = row.replace("\r", "");
        let re = Regex::new(r"\s+").unwrap();
        let row = re.replace_all(&row, " ").to_string();
        // Extract last 'word' as rank and save the rest as the title
        let mut words: Vec<&str> = row.split(" ").collect();
        let rank = words[words.len() - 1].parse::<u64>().unwrap();
        words.remove(words.len() - 1);
        let title = words.join(" ");

        results.push((title, rank));
    }
    Some(results)
}

// Search and Result structs to (de)serialize rust and website datatypes
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

// Define functional parts of the search page
async fn search(tera: web::Data<Tera>) -> impl Responder {
    let mut data = Context::new();
    data.insert("title", "Search field");
    let rendered = tera.render("search.html", &data).unwrap();
    HttpResponse::Ok().body(rendered)
}

// Define functional parts of the result page
async fn result(tera: web::Data<Tera>, data: web::Form<Search>) -> impl Responder {
    let mut page_data = Context::new();
    let mut results: Vec<Result> = Vec::new();
    // Run code generator with the string from the search field
    match run_code_gen(data.search.clone(), PATH_SQL) {
        // If code generator returns no error execute SQL and read the results
        Ok(_) => {
            execute_sql(PATH_SQL, PATH_RESULTS);
            let results_vec = read_results(PATH_RESULTS);
            // Fit search results into Result struct to properly display on the page, otherwise diplay error
            match results_vec {
                Some(results_vec) => {
                    for result in results_vec {
                        results.push(Result {
                            title: result.0.clone(),
                            rank: result.1,
                            // link to the Wikipedia article is also provided, whitespaces need to be replaced
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
        // If code generator returns error, display error instead of search results
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
