use std::slice::Iter;
use thiserror::Error;

use crate::ast::{Expression, Operator, Statement};

pub fn generate(ast: Vec<Statement>) -> Result<String, GenerateError> {
    let mut generator = Generator::new(ast.iter());
    generator.write();
    generator.write();
    let mut sql_parts: Vec<String> = Vec::new();
    sql_parts.push(
        "USE Wikipedia;
        SELECT *
        FROM [dbo].[Real_Article] AS FT_TBL
        INNER JOIN
        "
        .to_owned(),
    );
    while let Some(sql_part) = generator.next()? {
        sql_parts.push(sql_part);
    }
    sql_parts.push(
        "AS KEY_TBL
        ON FT_TBL.[ID] = KEY_TBL.[KEY]
        WHERE KEY_TBL.RANK > 2
        ORDER BY KEY_TBL.RANK DESC;"
            .to_owned(),
    );
    let sql = sql_parts.join(" ");
    Ok(sql)
}

struct Generator<'p> {
    statements: Iter<'p, Statement>,
    current: Statement,
    peek: Statement,
}

impl<'p> Generator<'p> {
    fn new(statements: Iter<'p, Statement>) -> Self {
        Self {
            statements,
            current: Statement::EoF,
            peek: Statement::EoF,
        }
    }

    fn next(&mut self) -> Result<Option<String>, GenerateError> {
        if self.current == Statement::EoF {
            return Ok(None);
        }
        Ok(Some(self.generate_statement()?))
    }

    fn write(&mut self) {
        self.current = self.peek.clone();
        self.peek = if let Some(statement) = self.statements.next() {
            statement.clone()
        } else {
            Statement::EoF
        };
    }

    fn generate_statement(&mut self) -> Result<String, GenerateError> {
        let sql: String = match &self.current {
            Statement::Expression { expression } => self.generate_expression(expression.clone())?,
            _ => return Err(GenerateError::UnexpectedStatement(self.current.clone())),
        };
        self.write();
        Ok(sql)
    }

    fn generate_expression(&mut self, expression: Expression) -> Result<String, GenerateError> {
        let sql: String = match expression {
            Expression::Identifier(s) => s,
            // What do with exact search?
            Expression::Exact(s) => s,
            Expression::Infix(expr1, operator, expr2) => {
                let sql_parts = [
                    self.generate_expression(*expr1)?,
                    self.generate_operator(operator)?,
                    self.generate_expression(*expr2)?,
                ];
                sql_parts.join(" ")
            }
            Expression::Prefix(operator, expr) => {
                let sql_parts = [
                    self.generate_operator(operator)?,
                    self.generate_expression(*expr)?,
                ];
                sql_parts.join(" ")
            }
            Expression::Function(name, expr) => {
                let sql_parts = match name.as_str() {
                    "contains" => [
                        "	CONTAINSTABLE(	[dbo].[Real_Article]
                            , [Text]
                            , '"
                        .to_owned(),
                        self.generate_expression(*expr)?,
                        "')".to_owned(),
                    ],
                    "freetext" => [
                        "	FREETEXTTABLE(	[dbo].[Real_Article]
                            , [Text]
                            , '"
                        .to_owned(),
                        self.generate_expression(*expr)?,
                        "')".to_owned(),
                    ],
                    _ => return Err(GenerateError::UnexpectedStatement(self.current.clone())),
                };
                sql_parts.join("")
            }
        };
        Ok(sql)
    }

    fn generate_operator(&mut self, operator: Operator) -> Result<String, GenerateError> {
        let op = match operator {
            Operator::And => "AND",
            Operator::Or => "OR",
            Operator::Not => "NOT",
        };
        Ok(op.to_owned())
    }
}

#[derive(Debug, Error)]
pub enum GenerateError {
    #[error("Unexpected statement {0:?}.")]
    UnexpectedStatement(Statement),
}
