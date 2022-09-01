use std::slice::Iter;
use thiserror::Error;

use crate::ast::*;
use crate::token::Token;

pub fn parse(tokens: Vec<Token>) -> Result<Program, ParseError> {
    let mut parser = Parser::new(tokens.iter());
    parser.read();
    parser.read();
    let mut program: Program = Vec::new();
    while let Some(statement) = parser.next()? {
        program.push(statement);
    }
    Ok(program)
}

#[derive(Debug, Clone, PartialEq, PartialOrd)]
enum Precedence {
    Lowest,
    Statement,
    Or,
    And,
    Equals,
    Not,
    Prefix,
    Group,
}

impl Precedence {
    fn token(token: Token) -> Self {
        match token {
            Token::Bang | Token::Minus => Self::Not,
            Token::Plus | Token::And => Self::And,
            Token::Or => Self::Or,
            Token::LeftParen => Self::Group,
            Token::Equals => Self::Equals,
            Token::At => Self::Statement,
            _ => Self::Lowest,
        }
    }
}

struct Parser<'p> {
    tokens: Iter<'p, Token>,
    current: Token,
    peek: Token,
}

impl<'p> Parser<'p> {
    fn new(tokens: Iter<'p, Token>) -> Self {
        Self {
            tokens,
            current: Token::EoF,
            peek: Token::EoF,
        }
    }

    fn next(&mut self) -> Result<Option<Statement>, ParseError> {
        if self.current == Token::EoF {
            return Ok(None);
        }
        Ok(Some(self.parse_statement()?))
    }

    fn read(&mut self) {
        self.current = self.peek.clone();
        self.peek = if let Some(token) = self.tokens.next() {
            token.clone()
        } else {
            Token::EoF
        };
    }

    fn current_is(&self, token: Token) -> bool {
        std::mem::discriminant(&self.current) == std::mem::discriminant(&token)
    }

    fn expect_token(&mut self, token: Token) -> Result<Token, ParseError> {
        if self.current_is(token) {
            Ok(self.current.clone())
        } else {
            Err(ParseError::UnexpectedToken(self.current.clone()))
        }
    }

    fn expect_token_and_read(&mut self, token: Token) -> Result<Token, ParseError> {
        let result = self.expect_token(token)?;
        self.read();
        Ok(result)
    }

    fn expect_identifier_and_read(&mut self) -> Result<Token, ParseError> {
        self.expect_token_and_read(Token::Identifier("".to_string()))
    }

    fn parse_statement(&mut self) -> Result<Statement, ParseError> {
        match self.current {
            _ => Ok(Statement::Expression {
                expression: self.parse_expression(Precedence::Lowest)?,
            }),
        }
    }

    fn parse_expression(&mut self, precedence: Precedence) -> Result<Expression, ParseError> {
        let mut expr: Expression = match self.current.clone() {
            Token::At => {
                let (name, search) = match self.parse_function()? {
                    Statement::Function { name, search, .. } => (name, search),
                    _ => return Err(ParseError::Unreachable),
                };
                Expression::Function(name, Box::new(search))
            }
            Token::String(s) => {
                self.expect_token_and_read(Token::String("".to_string()))?;
                Expression::String(s.to_string())
            }
            Token::Number(n) => {
                self.expect_token_and_read(Token::Number(0.0))?;
                Expression::Number(n)
            }
            Token::True => {
                self.expect_token_and_read(Token::True)?;
                Expression::Bool(true)
            }
            Token::False => {
                self.expect_token_and_read(Token::False)?;
                Expression::Bool(false)
            }
            Token::Identifier(s) => {
                self.expect_identifier_and_read()?;
                Expression::Identifier(s)
            }
            t @ Token::Minus | t @ Token::Bang => {
                self.expect_token_and_read(t.clone())?;
                Expression::Prefix(
                    Operator::token(t),
                    Box::new(self.parse_expression(Precedence::Prefix)?),
                )
            }
            Token::LeftParen => {
                let group_expression = match self.parse_group()? {
                    Statement::Group { expression } => expression,
                    _ => return Err(ParseError::Unreachable),
                };
                group_expression
            }
            _ => return Err(ParseError::UnexpectedToken(self.current.clone())),
        };
        while !self.current_is(Token::EoF) && precedence < Precedence::token(self.current.clone()) {
            if let Some(expression) = self.parse_infix_expression(expr.clone())? {
                expr = expression
            } else {
                break;
            }
        }
        Ok(expr)
    }

    fn parse_infix_expression(
        &mut self,
        expr: Expression,
    ) -> Result<Option<Expression>, ParseError> {
        Ok(match self.current {
            Token::Plus | Token::And | Token::Or => {
                let token = self.current.clone();
                self.read();
                let sec_expr = self.parse_expression(Precedence::token(token.clone()))?;
                Some(Expression::Infix(
                    Box::new(expr),
                    Operator::token(token),
                    Box::new(sec_expr),
                ))
            }
            Token::Equals => {
                self.read();
                let sec_expr = self.parse_expression(Precedence::Lowest)?;
                Some(Expression::Assign(Box::new(expr), Box::new(sec_expr)))
            }
            _ => None,
        })
    }

    fn parse_function(&mut self) -> Result<Statement, ParseError> {
        self.expect_token_and_read(Token::At)?;
        let name: Identifier = self.expect_identifier_and_read()?.into();
        self.expect_token_and_read(Token::Colon)?;
        let search: Expression = self.parse_expression(Precedence::Statement)?;
        self.expect_token_and_read(Token::Colon)?;
        Ok(Statement::Function { name, search })
    }

    fn parse_group(&mut self) -> Result<Statement, ParseError> {
        self.expect_token_and_read(Token::LeftParen)?;
        let expression = self.parse_expression(Precedence::Statement)?;
        self.expect_token_and_read(Token::RightParen)?;
        Ok(Statement::Group { expression })
    }
}

#[derive(Debug, Error)]
pub enum ParseError {
    #[error("Unexpected token {0:?}.")]
    UnexpectedToken(Token),
    #[error("Entered unreachable code.")]
    Unreachable,
}
