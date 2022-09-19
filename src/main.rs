use regex::Regex;
use std::fs::{read_to_string, File};
use std::io::Write;
use std::process::Command;

mod code_gen;

// Path Variables
const PATH_SEARCH: &str = "files\\search.txt";
const PATH_SQL: &str = "files\\fulltext.sql";
const PATH_RESULTS: &str = "files\\results.txt";

fn main() {
    let contents = read_to_string(PATH_SEARCH).unwrap();
    println!("Generator Errors: {:?}", run_code_gen(contents, PATH_SQL));
    println!("SQL EXECUTE {:?}", execute_sql(PATH_SQL, PATH_RESULTS));
    println!("{:?}", read_results(PATH_RESULTS));
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

fn read_results(path: &str) -> Option<Vec<(String, i64)>> {
    let contents = read_to_string(path).unwrap();
    let mut vec: Vec<&str> = contents.split("\n").collect();

    if vec.len() < 6 {
        return None;
    }

    // Remove metadata rows (first and last three rows)
    vec.remove(0);
    vec.remove(0);
    vec.remove(0);
    vec.remove(vec.len() - 1);
    vec.remove(vec.len() - 1);
    vec.remove(vec.len() - 1);

    let mut results: Vec<(String, i64)> = Vec::new();
    for row in vec {
        let row = row.replace("\r", "");
        let re = Regex::new(r"\s+").unwrap();
        let row = re.replace_all(&row, " ").to_string();

        let mut words: Vec<&str> = row.split(" ").collect();
        let rank = words[words.len() - 1].parse::<i64>().unwrap();
        words.remove(words.len() - 1);
        let title = words.join(" ");

        results.push((title, rank));
    }
    Some(results)
}
