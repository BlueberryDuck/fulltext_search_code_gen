use crate::token::Token;

pub type Program = Vec<Statement>;
pub type Identifier = String;

#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
    Expression { expression: Expression },
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expression {
    String(String),
    Number(f64),
    Bool(bool),
    Identifier(Identifier),
    Assign(Box<Expression>, Box<Expression>),
    Infix(Box<Expression>, Operator, Box<Expression>),
    Prefix(Operator, Box<Expression>),
    Call(Box<Expression>, Vec<Expression>),
    Group(Box<Expression>),
}

impl Expression {
    pub fn boxed(self) -> Box<Self> {
        Box::new(self)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Operator {
    Add,
    Subtract,
    And,
    Or,
}

impl Operator {
    pub fn token(token: Token) -> Self {
        match token {
            Token::Plus => Self::Add,
            Token::Minus => Self::Subtract,
            Token::And => Self::And,
            Token::Or => Self::Or,
            _ => unreachable!("{:?}", token),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Parameter {
    pub name: String,
}
