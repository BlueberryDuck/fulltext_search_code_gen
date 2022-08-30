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
    Assign,
    Or,
    And,
    Sum,
    Product,
    Prefix,
    Call,
}

impl Precedence {
    fn token(token: Token) -> Self {
        match token {
            Token::Asterisk | Token::Slash => Self::Product,
            Token::Plus | Token::Minus => Self::Sum,
            Token::LeftParen => Self::Call,
            Token::And => Self::And,
            Token::Or => Self::Or,
            Token::Assign => Self::Assign,
            Token::LeftBrace => Self::Statement,
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
            Token::Let => self.parse_let(),
            Token::Fn => self.parse_fn(true),
            Token::If => self.parse_if(),
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
            Token::Fn => {
                let (params, body) = match self.parse_fn(false)? {
                    Statement::FunctionDeclaration { params, body, .. } => (params, body),
                    _ => return Err(ParseError::Unreachable),
                };
                Expression::Closure(params, body)
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

                    if self.current_is(Token::Comma) {
                        self.read();
                    }
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
            Token::Plus
            | Token::Minus
            | Token::Asterisk
            | Token::Slash
            | Token::And
            | Token::Or => {
                let token = self.current.clone();
                self.read();
                let sec_expr = self.parse_expression(Precedence::token(token.clone()))?;
                Some(Expression::Infix(
                    Box::new(expr),
                    Operator::token(token),
                    Box::new(sec_expr),
                ))
            }
            Token::Assign => {
                self.read();
                let sec_expr = self.parse_expression(Precedence::Lowest)?;
                Some(Expression::Assign(Box::new(expr), Box::new(sec_expr)))
            }
            _ => None,
        })
    }

    fn parse_let(&mut self) -> Result<Statement, ParseError> {
        self.expect_token_and_read(Token::Let)?;
        let name: Identifier = self.expect_identifier_and_read()?.into();
        let initial: Option<Expression> = if self.current_is(Token::Assign) {
            self.expect_token_and_read(Token::Assign)?;
            Some(self.parse_expression(Precedence::Lowest)?)
        } else {
            None
        };
        Ok(Statement::LetDeclaration { name, initial })
    }

    fn parse_fn(&mut self, with_identifier: bool) -> Result<Statement, ParseError> {
        self.expect_token_and_read(Token::Fn)?;
        let name: Identifier = if with_identifier {
            self.expect_identifier_and_read()?.into()
        } else {
            String::from("<Closure>")
        };
        self.expect_token_and_read(Token::LeftParen)?;
        let mut params: Vec<Parameter> = Vec::new();
        while !self.current_is(Token::RightParen) {
            if self.current_is(Token::Comma) {
                self.expect_token_and_read(Token::Comma)?;
            }
            let param: String = self.expect_identifier_and_read()?.into();
            params.push(Parameter { name: param })
        }
        self.expect_token_and_read(Token::RightParen)?;
        let body: Vec<Statement> = self.parse_block()?;
        Ok(Statement::FunctionDeclaration { name, params, body })
    }

    fn parse_if(&mut self) -> Result<Statement, ParseError> {
        self.expect_token_and_read(Token::If)?;
        let condition = self.parse_expression(Precedence::Lowest)?;
        let then = self.parse_block()?;
        let otherwise = if self.current_is(Token::Else) {
            self.expect_token_and_read(Token::Else)?;
            Some(self.parse_block()?)
        } else {
            None
        };
        Ok(Statement::If {
            condition,
            then,
            otherwise,
        })
    }

    fn parse_block(&mut self) -> Result<Block, ParseError> {
        self.expect_token_and_read(Token::LeftBrace)?;
        let mut block = Vec::new();
        while !self.current_is(Token::RightBrace) {
            block.push(self.parse_statement()?);
        }
        self.expect_token_and_read(Token::RightBrace)?;
        Ok(block)
    }
}

#[derive(Debug, Error)]
pub enum ParseError {
    #[error("Unexpected token {0:?}.")]
    UnexpectedToken(Token),
    #[error("Entered unreachable code.")]
    Unreachable,
}

impl ParseError {
    /*pub fn print(self) {
        eprintln!("{}", self);
    }*/
}
