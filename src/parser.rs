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
    /* Statement, */
    Equals,
    Or,
    And,
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
            t @ Token::Minus => {
                self.expect_token_and_read(t.clone())?;
                Expression::Prefix(
                    Operator::token(t),
                    self.parse_expression(Precedence::Prefix)?.boxed(),
                )
            }
            Token::LeftParen => {
                self.expect_token_and_read(Token::LeftParen)?;
                let group_expression = self.parse_expression(Precedence::Lowest)?;
                self.expect_token_and_read(Token::RightParen)?;
                Expression::Group(group_expression.boxed())
            }
            _ => return Err(ParseError::UnexpectedToken(self.current.clone())),
        };
        while !self.current_is(Token::EoF) && precedence < Precedence::token(self.current.clone()) {
            if let Some(expression) = self.parse_postfix_expression(expr.clone())? {
                expr = expression;
            } else if let Some(expression) = self.parse_infix_expression(expr.clone())? {
                expr = expression
            } else {
                break;
            }
        }
        Ok(expr)
    }

    fn parse_postfix_expression(
        &mut self,
        expr: Expression,
    ) -> Result<Option<Expression>, ParseError> {
        Ok(match self.current {
            Token::LeftParen => {
                self.expect_token_and_read(Token::LeftParen)?;
                let mut args = Vec::new();
                while !self.current_is(Token::RightParen) {
                    args.push(self.parse_expression(Precedence::Lowest)?);
                }
                self.expect_token_and_read(Token::RightParen)?;
                Some(Expression::Call(Box::new(expr), args))
            }
            _ => None,
        })
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
}

#[derive(Debug, Error)]
pub enum ParseError {
    #[error("Unexpected token {0:?}.")]
    UnexpectedToken(Token),
}
