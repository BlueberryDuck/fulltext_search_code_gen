use std::slice::Iter;
use thiserror::Error;

use crate::ast::{Expression, Statement};
use crate::token::Token;

pub fn generate(program: Vec<Statement>) -> Result<String, GenerateError> {
    let mut generator = Generator::new(program.iter());
    generator.write();
    generator.write();
    let mut sql: String = String::new();
    while let Some(sql_part) = generator.next()? {
        sql.push_str(&sql_part);
    }
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

    fn current_is(&self, statement: Statement) -> bool {
        std::mem::discriminant(&self.current) == std::mem::discriminant(&statement)
    }

    fn generate_statement(&mut self) -> Result<String, GenerateError> {
        match self.current {
            Statement::Expression { .. } => Ok(self.generate_expression()?),
            _ => Ok("".to_owned()),
        }
    }

    fn generate_expression(&mut self) -> Result<String, GenerateError> {
        let mut sql: String = match self.current.clone() {
            _ => return Err(GenerateError::UnexpectedStatement(self.current.clone())),
        };
    }
}

#[derive(Debug, Error)]
pub enum GenerateError {
    #[error("Unexpected statement {0:?}.")]
    UnexpectedStatement(Statement),
}
