use logos::{Lexer, Logos};

pub fn lex(input: &str) -> Vec<Token> {
    Token::lexer(input).collect()
}

fn to_string(lex: &mut Lexer<Token>) -> Option<String> {
    let mut string = lex.slice().to_string();
    if string.starts_with('"') && string.ends_with('"') {
        string.remove(0);
        string.remove(string.len() - 1);
    }
    Some(string)
}

fn to_float(lex: &mut Lexer<Token>) -> Option<f64> {
    Some(lex.slice().parse().ok()?)
}

#[derive(Debug, Clone, Logos, PartialEq)]
pub enum Token {
    #[regex(r"[a-zA-Zß?üÜöÖäÄ,;._<>´`#§$%/\\=€]+", to_string)]
    Identifier(String),
    #[regex(r"[0-9]+(\.[0-9]+)?", to_float)]
    Number(f64),
    #[regex(r##""(?:[^"\\]|\\.)*""##, to_string)]
    String(String),

    #[token("(")]
    LeftParen,
    #[token(")")]
    RightParen,

    #[token("!")]
    Bang,
    #[token("-")]
    Minus,

    #[token("&")]
    And,
    #[token("+")]
    Plus,
    #[token("|")]
    Or,

    #[token("@")]
    At,
    #[token(":")]
    Colon,

    EoF,

    #[error]
    #[regex(r"[\s\t\n\f]+", logos::skip)]
    Error,
}

impl Into<String> for Token {
    fn into(self) -> String {
        match self {
            Token::Identifier(s) => s,
            Token::String(s) => s,
            _ => unreachable!(),
        }
    }
}
