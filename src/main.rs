use regex::Regex;
use std::env::args;
use std::fs::{read_to_string, File};
use std::io::Write;

mod ast;
mod generator;
mod parser;
mod token;

fn main() -> std::io::Result<()> {
    let file = args().nth(1).unwrap();
    let contents = read_to_string(file).unwrap();

    let tokens = token::lex(contents.as_str());
    let ast = parser::parse(tokens);
    let generator = generator::generate(ast.unwrap());

    let sql_file = "C:\\_GIT\\fulltext_search_code_gen\\examples\\example.sql";
    let mut output = File::create(sql_file)?;

    println!("{:?}", read_results("examples\\output.txt"));

    write!(output, "{}", generator.unwrap())
}

fn read_results(path: &str) -> Vec<(String, i64)> {
    let contents = read_to_string(path).unwrap();
    let mut vec: Vec<&str> = contents.split("\n").collect();

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
    results
}
