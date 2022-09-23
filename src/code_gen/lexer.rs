use logos::{Lexer, Logos};

pub fn lex(input: &str) -> Vec<Token> {
    Token::lexer(input).collect()
}

fn to_string(lex: &mut Lexer<Token>) -> Option<String> {
    let string = lex.slice().to_string();
    Some(string)
}

fn to_float(lex: &mut Lexer<Token>) -> Option<f64> {
    Some(lex.slice().parse().ok()?)
}

fn to_u64(lex: &mut Lexer<Token>) -> Option<u64> {
    Some(lex.slice().parse().ok()?)
}

#[derive(Debug, Clone, Logos, PartialEq)]
pub enum Token {
    #[regex(r##""(?:[^"\\]|\\.)*"|[a-zA-Zß?üÜöÖäÄ;\._<>´`#§$%/\\=€]+"##, to_string)]
    WordOrPhrase(String),
    #[regex(r"0+(\.[0-9]+)?|1", to_float)]
    ZeroToOne(f64),
    #[regex(r"[0-9]+", to_u64)]
    Number(u64),

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
    #[token("(")]
    LeftParen,
    #[token(")")]
    RightParen,
    #[token(",")]
    Comma,

    #[token("@contains")]
    Contains,
    #[token("@startswith")]
    Starts,
    #[token("@inflection")]
    Inflection,
    #[token("@thesaurus")]
    Thesaurus,
    #[token("@near")]
    Near,
    #[token("@weighted")]
    Weighted,

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
            Token::WordOrPhrase(s) => s,
            _ => unreachable!(),
        }
    }
}
