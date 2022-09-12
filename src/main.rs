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
    write!(output, "{}", generator.unwrap())
}
