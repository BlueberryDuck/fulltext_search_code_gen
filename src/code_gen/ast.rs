use crate::code_gen::lexer::Token;

#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
    Group {
        expression: Expression,
    },
    Infix {
        statement: Box<Statement>,
        operator: Operator,
        second_statement: Box<Statement>,
    },
    Contains {
        expression: Expression,
    },
    Starts {
        expression: Expression,
    },
    Inflection {
        expression: Expression,
    },
    Thesaurus {
        expression: Expression,
    },
    Near {
        parameter: Vec<Expression>,
        proximity: Expression,
    },
    Weighted {
        parameter: Vec<(Expression, Expression)>,
    },
    EoF,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expression {
    WordOrPhrase(String),
    Number(u64),
    ZeroToOne(f64),
    Infix(Box<Expression>, Operator, Box<Expression>),
    Prefix(Operator, Box<Expression>),
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
