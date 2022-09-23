use std::slice::Iter;
use thiserror::Error;

use crate::code_gen::ast::{Expression, Operator, Statement};

// Database constants
const DB_NAME: &str = "Wikipedia";
const TBL_NAME: &str = "[dbo].[Real_Article]";
const RETURN_ATTRIBUTE: &str = "Title";
const TOP_ROWS: u64 = 5;

pub fn generate(ast: Vec<Statement>) -> Result<String, GenerateError> {
    let mut generator = Generator::new(ast.iter());
    generator.write();
    generator.write();

    let mut sql_parts: Vec<String> = Vec::new();
    sql_parts.push(format!(
        "USE {}; SELECT TOP {} * FROM(SELECT FT_TBL.{}, KEY_TBL.RANK FROM {} AS FT_TBL INNER JOIN",
        DB_NAME, TOP_ROWS, RETURN_ATTRIBUTE, TBL_NAME
    ));
    while let Some(sql_part) = generator.next()? {
        sql_parts.push(sql_part);
    }
    sql_parts.push("AS KEY_TBL ON FT_TBL.[ID] = KEY_TBL.[KEY] WHERE KEY_TBL.RANK > 5) AS FS_RESULT ORDER BY FS_RESULT.RANK DESC;".to_owned());
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
            Expression::WordOrPhrase(s) => s,
            Expression::Number(u) => u.to_string(),
            Expression::ZeroToOne(f) => f.to_string(),
            Expression::Infix(expr1, operator, expr2) => {
                let sql_parts = [
                    String::from("("),
                    self.generate_expression(*expr1)?,
                    String::from(")"),
                    self.generate_operator(operator)?,
                    String::from("("),
                    self.generate_expression(*expr2)?,
                    String::from(")"),
                ];
                sql_parts.join(" ")
            }
            Expression::Prefix(operator, expr) => {
                let sql_parts = [
                    self.generate_operator(operator)?,
                    String::from("("),
                    self.generate_expression(*expr)?,
                    String::from(")"),
                ];
                sql_parts.join(" ")
            }
            Expression::Contains(expr) => {
                format!(
                    "CONTAINSTABLE({}, *, '{}')",
                    TBL_NAME,
                    self.generate_expression(*expr)?
                )
            }
            Expression::Starts(expr) => {
                let mut word_or_phrase = self.generate_expression(*expr)?;
                if word_or_phrase.ends_with("\"") {
                    word_or_phrase.insert(word_or_phrase.len() - 1, '*');
                } else {
                    word_or_phrase.push('*');
                }

                format!("CONTAINSTABLE({}, *, '{}')", TBL_NAME, word_or_phrase)
            }
            Expression::Inflection(expr) => {
                let mut word_or_phrase = self.generate_expression(*expr)?;
                if word_or_phrase.starts_with('"') && word_or_phrase.ends_with('"') {
                    word_or_phrase.remove(0);
                    word_or_phrase.remove(word_or_phrase.len() - 1);
                }
                format!(
                    "CONTAINSTABLE({}, *, 'FORMSOF(INFLECTIONAL,\"{}\")')",
                    TBL_NAME, word_or_phrase
                )
            }
            Expression::Thesaurus(expr) => {
                let mut word_or_phrase = self.generate_expression(*expr)?;
                if word_or_phrase.starts_with('"') && word_or_phrase.ends_with('"') {
                    word_or_phrase.remove(0);
                    word_or_phrase.remove(word_or_phrase.len() - 1);
                }
                format!(
                    "CONTAINSTABLE({}, *, 'FORMSOF(THESAURUS,\"{}\")')",
                    TBL_NAME, word_or_phrase
                )
            }
            Expression::Near(parameter, proximity) => {
                let mut sql_parts: Vec<String> = Vec::new();
                sql_parts.push(format!("CONTAINSTABLE({}, *, 'NEAR((", TBL_NAME));
                for expression in parameter {
                    let string = self.generate_expression(expression)?;
                    sql_parts.push(format!("{}", string));
                    sql_parts.push(String::from(", "));
                }
                sql_parts.remove(sql_parts.len() - 1);
                sql_parts.push(format!("), {})')", self.generate_expression(*proximity)?));
                sql_parts.join("")
            }
            Expression::Weighted(parameter) => {
                let mut sql_parts: Vec<String> = Vec::new();
                sql_parts.push(format!("CONTAINSTABLE({}, *, 'ISABOUT(", TBL_NAME));
                for (word_or_phrase_expr, weight_expr) in parameter {
                    let word_or_phrase = self.generate_expression(word_or_phrase_expr)?;
                    let weight = self.generate_expression(weight_expr)?;
                    sql_parts.push(format!("{} WEIGHT({})", word_or_phrase, weight));
                    sql_parts.push(String::from(", "));
                }
                sql_parts.remove(sql_parts.len() - 1);
                sql_parts.push(String::from(")')"));
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
