use std::slice::Iter;
use thiserror::Error;

use crate::code_gen::ast::*;
use crate::code_gen::lexer::Token;

// Main function to start parsing process
// Input: vec of tokens
// Ouput: abstract syntax tree (vec of statements)
pub fn parse(tokens: Vec<Token>) -> Result<Vec<Statement>, ParseError> {
    let mut parser = Parser::new(tokens.iter());
    // read twice to overwrite intial EoF tokens
    parser.read();
    parser.read();
    let mut ast: Vec<Statement> = Vec::new();
    while let Some(statement) = parser.next()? {
        ast.push(statement);
    }
    Ok(ast)
}

// Precedence to enable priorities between expressions
// Example: this OR that AND some (AND should have a higher priority)
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

// Match tokens to precedences
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

// Parser saves current and next tokens as attribute
struct Parser<'p> {
    tokens: Iter<'p, Token>,
    current: Token,
    peek: Token,
}

impl<'p> Parser<'p> {
    // Initial parser creation
    fn new(tokens: Iter<'p, Token>) -> Self {
        Self {
            tokens,
            current: Token::EoF,
            peek: Token::EoF,
        }
    }

    // Parse next statement if possible
    // Output: statement or error
    fn next(&mut self) -> Result<Option<Statement>, ParseError> {
        if self.current == Token::EoF {
            return Ok(None);
        }
        Ok(Some(self.parse_statement(Precedence::Lowest)?))
    }

    // Set current and peek one step further in the vec of tokens
    fn read(&mut self) {
        self.current = self.peek.clone();
        self.peek = if let Some(token) = self.tokens.next() {
            token.clone()
        } else {
            Token::EoF
        };
    }

    // See what the current token is
    // Output: boolean
    fn current_is(&self, token: Token) -> bool {
        std::mem::discriminant(&self.current) == std::mem::discriminant(&token)
    }

    // Current token should match the one given
    // Input: token
    // Output: token or error
    fn expect_token(&mut self, token: Token) -> Result<Token, ParseError> {
        if self.current_is(token) {
            Ok(self.current.clone())
        } else {
            Err(ParseError::UnexpectedToken(self.current.clone()))
        }
    }

    // Current token should match the one given and read to next token
    // Input: token
    // Output: token or error
    fn expect_token_and_read(&mut self, token: Token) -> Result<Token, ParseError> {
        let result = self.expect_token(token)?;
        self.read();
        Ok(result)
    }

    // Parse statement, can only be a function or combination of functions
    // Input: precedence
    // Output: statement or error
    fn parse_statement(&mut self, precedence: Precedence) -> Result<Statement, ParseError> {
        let mut statement = match self.current.clone() {
            Token::Contains => Statement::Contains {
                expression: self.parse_contains()?,
            },
            Token::Starts => Statement::Starts {
                expression: self.parse_starts()?,
            },
            Token::Inflection => Statement::Inflection {
                expression: self.parse_inflection()?,
            },
            Token::Thesaurus => Statement::Thesaurus {
                expression: self.parse_thesaurus()?,
            },
            Token::Near => {
                let (parameter, proximity) = self.parse_near()?;
                Statement::Near {
                    parameter,
                    proximity,
                }
            }
            Token::Weighted => Statement::Weighted {
                parameter: self.parse_weighted()?,
            },
            _ => return Err(ParseError::UnexpectedToken(self.current.clone())),
        };
        // After a function could be an infix operator
        while !self.current_is(Token::EoF) && precedence < Precedence::token(self.current.clone()) {
            if let Some(in_statement) = self.parse_infix_statement(statement.clone())? {
                statement = in_statement
            } else {
                break;
            }
        }
        Ok(statement)
    }

    // Parse expression, could be a search term, number, operator or combination of epxressions
    fn parse_expression(&mut self, precedence: Precedence) -> Result<Expression, ParseError> {
        let mut expr = match self.current.clone() {
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
            // Start a group which gets higher precedence
            Token::LeftParen => {
                let group_expression = match self.parse_group()? {
                    Statement::Group { expression } => expression,
                    _ => return Err(ParseError::Unreachable),
                };
                group_expression
            }
            _ => return Err(ParseError::UnexpectedToken(self.current.clone())),
        };
        // Afer an expression could be an infix operator or directly a new expression (here called postfix operator)
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

    // Postfix operator is called when two expressions are read, automatically inserting an AND inbetween
    // Second Expression could have an NOT operator before the actual expression
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

    // Infix operators AND and OR expect an expression on either side
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

    // Infix operators AND and OR expect a statement on either side
    fn parse_infix_statement(
        &mut self,
        statement: Statement,
    ) -> Result<Option<Statement>, ParseError> {
        Ok(match self.current {
            Token::Plus | Token::And | Token::Or => {
                let token = self.current.clone();
                self.read();
                let second_statement = self.parse_statement(Precedence::token(token.clone()))?;
                Some(Statement::Infix {
                    statement: Box::new(statement),
                    operator: Operator::token(token),
                    second_statement: Box::new(second_statement),
                })
            }
            _ => None,
        })
    }

    // Functions all have a similar strucure needing colons to surround their parameters

    // Contains function only expects one word or phrase or combination of expressions
    fn parse_contains(&mut self) -> Result<Expression, ParseError> {
        self.expect_token_and_read(Token::Contains)?;
        self.expect_token_and_read(Token::Colon)?;
        let expression: Expression = self.parse_expression(Precedence::Statement)?;
        self.expect_token_and_read(Token::Colon)?;
        Ok(expression)
    }

    // Startswith function only expects one one word or phrase or combination of expressions
    fn parse_starts(&mut self) -> Result<Expression, ParseError> {
        self.expect_token_and_read(Token::Starts)?;
        self.expect_token_and_read(Token::Colon)?;
        let expression: Expression = self.parse_expression(Precedence::Statement)?;
        self.expect_token_and_read(Token::Colon)?;
        Ok(expression)
    }

    // Inflection function only expects one one word or phrase or combination of expressions
    fn parse_inflection(&mut self) -> Result<Expression, ParseError> {
        self.expect_token_and_read(Token::Inflection)?;
        self.expect_token_and_read(Token::Colon)?;
        let expression: Expression = self.parse_expression(Precedence::Statement)?;
        self.expect_token_and_read(Token::Colon)?;
        Ok(expression)
    }

    // Thesaurus function only expects one one word or phrase or combination of expressions
    fn parse_thesaurus(&mut self) -> Result<Expression, ParseError> {
        self.expect_token_and_read(Token::Thesaurus)?;
        self.expect_token_and_read(Token::Colon)?;
        let expression: Expression = self.parse_expression(Precedence::Statement)?;
        self.expect_token_and_read(Token::Colon)?;
        Ok(expression)
    }

    // Near function expects multiple comma-seperated words or phrases with an optional number as the last parameter
    fn parse_near(&mut self) -> Result<(Vec<Expression>, Expression), ParseError> {
        self.expect_token_and_read(Token::Near)?;
        self.expect_token_and_read(Token::Colon)?;
        let mut parameter: Vec<Expression> = Vec::new();
        // Proximity has a default value of 5 if no number is given
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
        Ok((parameter, proximity))
    }

    // Weighted function expects pairs of words or phrases and a weight between 0 and 1
    // All weights must add up to exactly 1
    fn parse_weighted(&mut self) -> Result<Vec<(Expression, Expression)>, ParseError> {
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
        Ok(parameter)
    }

    // Groups must encapsulate an expression with parenthesis and have higher precedence then other operators
    fn parse_group(&mut self) -> Result<Statement, ParseError> {
        self.expect_token_and_read(Token::LeftParen)?;
        let expression = self.parse_expression(Precedence::Statement)?;
        self.expect_token_and_read(Token::RightParen)?;
        Ok(Statement::Group { expression })
    }
}

// Types of errors covered by the parser
#[derive(Debug, Error)]
pub enum ParseError {
    #[error("Unexpected token {0:?}.")]
    UnexpectedToken(Token),
    #[error("Entered unreachable code.")]
    Unreachable,
    #[error("Weights do not add up to 1.0. Sum of all weights: {0}")]
    WeightError(f64),
}
