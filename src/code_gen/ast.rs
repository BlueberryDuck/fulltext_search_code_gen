use crate::code_gen::lexer::Token;

#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
    Group { expression: Expression },
    Contains { expression: Expression },
    Starts { expression: Expression },
    Inflection { expression: Expression },
    Thesaurus { expression: Expression },
    Expression { expression: Expression },
    Near { parameter: Vec<Expression> },
    EoF,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expression {
    WordOrPhrase(String),
    Number(u64),
    Infix(Box<Expression>, Operator, Box<Expression>),
    Prefix(Operator, Box<Expression>),
    Contains(Box<Expression>),
    Starts(Box<Expression>),
    Inflection(Box<Expression>),
    Thesaurus(Box<Expression>),
    Near(Vec<Expression>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Operator {
    And,
    Or,
    Not,
}

impl Operator {
    pub fn token(token: Token) -> Self {
        match token {
            Token::And | Token::Plus => Self::And,
            Token::Or => Self::Or,
            Token::Minus | Token::Bang => Self::Not,
            _ => unreachable!("{:?}", token),
        }
    }
}
