use std::env::args;
use std::fs::read_to_string;

mod ast;
mod generator;
mod parser;
mod token;

fn main() {
    let file = args().nth(1).unwrap();
    let contents = read_to_string(file).unwrap();

    let tokens = token::generate(contents.as_str());
    let ast = parser::parse(tokens);

    println!("{:?}", ast);
}
