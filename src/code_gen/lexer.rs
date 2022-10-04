use logos::{Lexer, Logos};

// Main function to start lexing process
// Input: string
// Output: vec of tokens
pub fn lex(input: &str) -> Vec<Token> {
    Token::lexer(input).collect()
}

// helper function to format strings
fn to_string(lex: &mut Lexer<Token>) -> Option<String> {
    let string = lex.slice().to_string();
    Some(string)
}

// helper function to format floats
fn to_float(lex: &mut Lexer<Token>) -> Option<f64> {
    Some(lex.slice().parse().ok()?)
}

// helper function to format unsigned integer
fn to_u64(lex: &mut Lexer<Token>) -> Option<u64> {
    Some(lex.slice().parse().ok()?)
}

// List of all tokens that are accepted by the language
#[derive(Debug, Clone, Logos, PartialEq)]
pub enum Token {
    // Regex: phrase starting and ending with " and escaped character \" or just a word allowing a list of special characters
    #[regex(r##""(?:[^"\\]|\\.)*"|[a-zA-Zß?üÜöÖäÄ;\._<>´`#§$%/\\=€]+"##, to_string)]
    WordOrPhrase(String),
    // Regex: any float between 0 and 1
    #[regex(r"0+(\.[0-9]+)?|1", to_float)]
    ZeroToOne(f64),
    // Regex: any postive integer
    #[regex(r"[0-9]+", to_u64)]
    Number(u64),
    // ! and - for NOT
    #[token("!")]
    Bang,
    #[token("-")]
    Minus,
    // & and + for AND
    #[token("&")]
    And,
    #[token("+")]
    Plus,
    // | for OR
    #[token("|")]
    Or,
    // Parentheses for grouping
    #[token("(")]
    LeftParen,
    #[token(")")]
    RightParen,
    // Comma for parameter separation
    #[token(",")]
    Comma,
    // Functions
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
    // Colon to surround functions parameters
    #[token(":")]
    Colon,
    // End of File
    EoF,
    // Error and skip whitespaces
    #[error]
    #[regex(r"[\s\t\n\f]+", logos::skip)]
    Error,
}

// Enable tokens to be casted as strings
impl Into<String> for Token {
    fn into(self) -> String {
        match self {
            Token::WordOrPhrase(s) => s,
            _ => unreachable!(),
        }
    }
}
