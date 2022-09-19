use crate::code_gen::lexer::Token;

pub type Identifier = String;

#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
    Group {
        expression: Expression,
    },
    Function {
        name: Identifier,
        search: Expression,
    },
    Expression {
        expression: Expression,
    },
    EoF,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expression {
    Exact(String),
    Identifier(Identifier),
    Infix(Box<Expression>, Operator, Box<Expression>),
    Prefix(Operator, Box<Expression>),
    Function(Identifier, Box<Expression>),
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
