use crate::token::Token;

pub type Program = Vec<Statement>;
pub type Identifier = String;
pub type Block = Vec<Statement>;

#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
    Expression {
        expression: Expression,
    },
    LetDeclaration {
        name: Identifier,
        initial: Option<Expression>,
    },
    FunctionDeclaration {
        name: Identifier,
        params: Vec<Parameter>,
        body: Block,
    },
    If {
        condition: Expression,
        then: Block,
        otherwise: Option<Block>,
    },
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
    Closure(Vec<Parameter>, Vec<Statement>),
    Group(Box<Expression>),
}

impl Expression {
    /*pub fn some(self) -> Option<Self> {
        Some(self)
    }*/
    pub fn boxed(self) -> Box<Self> {
        Box::new(self)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Operator {
    Add,
    Subtract,
    Multiply,
    Divide,
    And,
    Or,
}

impl Operator {
    pub fn token(token: Token) -> Self {
        match token {
            Token::Plus => Self::Add,
            Token::Minus => Self::Subtract,
            Token::Asterisk => Self::Multiply,
            Token::Slash => Self::Divide,
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
