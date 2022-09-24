use std::slice::Iter;
use thiserror::Error;

use crate::code_gen::ast::*;
use crate::code_gen::lexer::Token;

pub fn parse(tokens: Vec<Token>) -> Result<Vec<Statement>, ParseError> {
    let mut parser = Parser::new(tokens.iter());
    parser.read();
    parser.read();
    let mut ast: Vec<Statement> = Vec::new();
    while let Some(statement) = parser.next()? {
        ast.push(statement);
    }
    Ok(ast)
}

#[derive(Debug, Clone, PartialEq, PartialOrd)]
enum Precedence {
    Lowest,
    Statement,
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
            Token::Plus | Token::And | Token::WordOrPhrase(..) => Self::And,
            Token::Or => Self::Or,
            Token::LeftParen => Self::Group,
            Token::Contains | Token::Starts | Token::Inflection => Self::Statement,
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

    fn parse_statement(&mut self) -> Result<Statement, ParseError> {
        match self.current {
            _ => Ok(Statement::Expression {
                expression: self.parse_expression(Precedence::Lowest)?,
            }),
        }
    }

    fn parse_expression(&mut self, precedence: Precedence) -> Result<Expression, ParseError> {
        let mut expr: Expression = match self.current.clone() {
            Token::Contains => {
                let expression = match self.parse_contains()? {
                    Statement::Contains { expression } => expression,
                    _ => return Err(ParseError::Unreachable),
                };
                Expression::Contains(Box::new(expression))
            }
            Token::Starts => {
                let expression = match self.parse_starts()? {
                    Statement::Starts { expression } => expression,
                    _ => return Err(ParseError::Unreachable),
                };
                Expression::Starts(Box::new(expression))
            }
            Token::Inflection => {
                let expression = match self.parse_inflection()? {
                    Statement::Inflection { expression } => expression,
                    _ => return Err(ParseError::Unreachable),
                };
                Expression::Inflection(Box::new(expression))
            }
            Token::Thesaurus => {
                let expression = match self.parse_thesaurus()? {
                    Statement::Thesaurus { expression } => expression,
                    _ => return Err(ParseError::Unreachable),
                };
                Expression::Thesaurus(Box::new(expression))
            }
            Token::Near => {
                let (parameter, proximity) = match self.parse_near()? {
                    Statement::Near {
                        parameter,
                        proximity,
                    } => (parameter, proximity),
                    _ => return Err(ParseError::Unreachable),
                };
                Expression::Near(parameter, Box::new(proximity))
            }
            Token::Weighted => {
                let parameter = match self.parse_weighted()? {
                    Statement::Weighted { parameter } => parameter,
                    _ => return Err(ParseError::Unreachable),
                };
                Expression::Weighted(parameter)
            }
            Token::WordOrPhrase(s) => {
                self.expect_token_and_read(Token::WordOrPhrase("".to_string()))?;
                Expression::WordOrPhrase(s.to_string())
            }
            Token::Number(u) => {
                self.expect_token_and_read(Token::Number(0))?;
                Expression::Number(u)
            }
            Token::ZeroToOne(f) => {
                self.expect_token_and_read(Token::ZeroToOne(0.0))?;
                Expression::ZeroToOne(f)
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
            Token::Minus | Token::Bang | Token::WordOrPhrase(..) => {
                let sec_expr = self.parse_expression(Precedence::And)?;
                Some(Expression::Infix(
                    Box::new(expr),
                    Operator::And,
                    Box::new(sec_expr),
                ))
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
            _ => None,
        })
    }

    fn parse_contains(&mut self) -> Result<Statement, ParseError> {
        self.expect_token_and_read(Token::Contains)?;
        self.expect_token_and_read(Token::Colon)?;
        let expression: Expression = self.parse_expression(Precedence::Statement)?;
        self.expect_token_and_read(Token::Colon)?;
        Ok(Statement::Contains { expression })
    }

    fn parse_starts(&mut self) -> Result<Statement, ParseError> {
        self.expect_token_and_read(Token::Starts)?;
        self.expect_token_and_read(Token::Colon)?;
        let expression: Expression = self.parse_expression(Precedence::Statement)?;
        self.expect_token_and_read(Token::Colon)?;
        Ok(Statement::Starts { expression })
    }

    fn parse_inflection(&mut self) -> Result<Statement, ParseError> {
        self.expect_token_and_read(Token::Inflection)?;
        self.expect_token_and_read(Token::Colon)?;
        let expression: Expression = self.parse_expression(Precedence::Statement)?;
        self.expect_token_and_read(Token::Colon)?;
        Ok(Statement::Inflection { expression })
    }

    fn parse_thesaurus(&mut self) -> Result<Statement, ParseError> {
        self.expect_token_and_read(Token::Thesaurus)?;
        self.expect_token_and_read(Token::Colon)?;
        let expression: Expression = self.parse_expression(Precedence::Statement)?;
        self.expect_token_and_read(Token::Colon)?;
        Ok(Statement::Thesaurus { expression })
    }

    fn parse_near(&mut self) -> Result<Statement, ParseError> {
        self.expect_token_and_read(Token::Near)?;
        self.expect_token_and_read(Token::Colon)?;
        let mut parameter: Vec<Expression> = Vec::new();
        let mut proximity = Expression::Number(5);
        while !self.current_is(Token::Colon) {
            if self.current_is(Token::Comma) {
                self.expect_token_and_read(Token::Comma)?;
            }
            match self.parse_expression(Precedence::Lowest)? {
                Expression::WordOrPhrase(s) => parameter.push(Expression::WordOrPhrase(s)),
                Expression::Number(u) => {
                    if self.current_is(Token::Colon) {
                        proximity = Expression::Number(u)
                    } else {
                        return Err(ParseError::UnexpectedToken(self.current.clone()));
                    }
                }
                _ => return Err(ParseError::UnexpectedToken(self.current.clone())),
            }
        }
        self.expect_token_and_read(Token::Colon)?;
        Ok(Statement::Near {
            parameter,
            proximity,
        })
    }

    fn parse_weighted(&mut self) -> Result<Statement, ParseError> {
        self.expect_token_and_read(Token::Weighted)?;
        self.expect_token_and_read(Token::Colon)?;
        let mut parameter: Vec<(Expression, Expression)> = Vec::new();
        let mut sum_weights: f64 = 0.0;
        while !self.current_is(Token::Colon) {
            if self.current_is(Token::Comma) {
                self.expect_token_and_read(Token::Comma)?;
            }
            let expression = match self.parse_expression(Precedence::Lowest)? {
                Expression::WordOrPhrase(s) => Expression::WordOrPhrase(s),
                _ => return Err(ParseError::UnexpectedToken(self.current.clone())),
            };
            self.expect_token_and_read(Token::Comma)?;
            let weight = match self.parse_expression(Precedence::Lowest)? {
                Expression::ZeroToOne(f) => {
                    sum_weights += f;
                    Expression::ZeroToOne(f)
                }
                _ => return Err(ParseError::UnexpectedToken(self.current.clone())),
            };
            parameter.push((expression, weight));
        }
        if sum_weights != 1.0 {
            return Err(ParseError::WeightError(sum_weights));
        }
        self.expect_token_and_read(Token::Colon)?;
        Ok(Statement::Weighted { parameter })
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
    #[error("Weights do not add up to 1.0. Sum of all weights: {0}")]
    WeightError(f64),
}
