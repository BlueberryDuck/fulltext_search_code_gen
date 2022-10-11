use std::slice::Iter;
use thiserror::Error;

use crate::code_gen::ast::{Expression, Operator, Statement};

// Database constants
const DB_NAME: &str = "Wikipedia";
const TBL_NAME: &str = "[dbo].[Real_Article]";
const RETURN_ATTRIBUTE: &str = "Title";
const TOP_ROWS: u64 = 5;

// Main function to start the generation process
// Input: vec of statements (ast)
// Output: string (sql statement)
pub fn generate(ast: Vec<Statement>) -> Result<String, GenerateError> {
    let mut generator = Generator::new(ast.iter());
    // write twice to overwrite initial EoF statements
    generator.write();
    generator.write();
    let mut sql_parts: Vec<String> = Vec::new();
    sql_parts.push(format!(
        "USE {}; SELECT TOP {} * FROM(SELECT FT_TBL.{}, KEY_TBL.RANK FROM {} AS FT_TBL INNER JOIN CONTAINSTABLE({}, *, '",
        DB_NAME, TOP_ROWS, RETURN_ATTRIBUTE, TBL_NAME, TBL_NAME
    ));
    // generate all functions as JOIN constraints
    while let Some(sql_part) = generator.next()? {
        sql_parts.push(sql_part);
    }
    sql_parts.push("') AS KEY_TBL ON FT_TBL.[ID] = KEY_TBL.[KEY] WHERE KEY_TBL.RANK > 5) AS FS_RESULT ORDER BY FS_RESULT.RANK DESC;".to_owned());
    Ok(sql_parts.join(" "))
}

// Generator struct with current and next statements as attributes
struct Generator<'p> {
    statements: Iter<'p, Statement>,
    current: Statement,
    peek: Statement,
}

impl<'p> Generator<'p> {
    // Initial generator creation
    fn new(statements: Iter<'p, Statement>) -> Self {
        Self {
            statements,
            current: Statement::EoF,
            peek: Statement::EoF,
        }
    }

    // Generate next statement if possible
    fn next(&mut self) -> Result<Option<String>, GenerateError> {
        if self.current == Statement::EoF {
            return Ok(None);
        }
        Ok(Some(self.generate_statement(self.current.clone())?))
    }

    // Set current and peek one step further in the ast
    fn write(&mut self) {
        self.current = self.peek.clone();
        self.peek = if let Some(statement) = self.statements.next() {
            statement.clone()
        } else {
            Statement::EoF
        };
    }

    // Generate statement, always a function or combination of functions
    // Input: statement to generate
    // Output: string
    fn generate_statement(&mut self, statement: Statement) -> Result<String, GenerateError> {
        let sql: String = match statement {
            Statement::Infix {
                statement,
                operator,
                second_statement,
            } => {
                let sql_parts = [
                    self.generate_statement(*statement)?,
                    self.generate_operator(operator)?,
                    self.generate_statement(*second_statement)?,
                ];
                sql_parts.join(" ")
            }
            // Contains generates it's search condition without mutation
            Statement::Contains { expression } => {
                format!("{}", self.generate_expression(expression)?)
            }
            // Startswith adds a * to end of a word or before the last " in a phrase
            Statement::Starts { expression } => {
                let mut word_or_phrase = self.generate_expression(expression)?;
                if word_or_phrase.starts_with('"') && word_or_phrase.ends_with('"') {
                    word_or_phrase.insert(word_or_phrase.len() - 1, '*');
                } else {
                    word_or_phrase.push('*');
                }
                format!("{}", word_or_phrase)
            }
            // Inflection calls the inflection function from MSSQL
            Statement::Inflection { expression } => {
                let mut word_or_phrase = self.generate_expression(expression)?;
                if word_or_phrase.starts_with('"') && word_or_phrase.ends_with('"') {
                    word_or_phrase.remove(0);
                    word_or_phrase.remove(word_or_phrase.len() - 1);
                }
                format!("FORMSOF(INFLECTIONAL,\"{}\")", word_or_phrase)
            }
            // Thesaurus calls the thesaurus function from MSSQL
            Statement::Thesaurus { expression } => {
                let mut word_or_phrase = self.generate_expression(expression)?;
                if word_or_phrase.starts_with('"') && word_or_phrase.ends_with('"') {
                    word_or_phrase.remove(0);
                    word_or_phrase.remove(word_or_phrase.len() - 1);
                }
                format!("FORMSOF(THESAURUS,\"{}\")", word_or_phrase)
            }
            // Near generates a parameter list of all search criteria and proximity in the end
            Statement::Near {
                parameter,
                proximity,
            } => {
                let mut sql_parts: Vec<String> = Vec::new();
                sql_parts.push(format!("NEAR(("));
                for expression in parameter {
                    let string = self.generate_expression(expression)?;
                    sql_parts.push(format!("{}", string));
                    sql_parts.push(String::from(", "));
                }
                sql_parts.remove(sql_parts.len() - 1);
                sql_parts.push(format!("), {})", self.generate_expression(proximity)?));
                sql_parts.join("")
            }
            // Weighted generates tuples of search criteria and their respective weight
            Statement::Weighted { parameter } => {
                let mut sql_parts: Vec<String> = Vec::new();
                sql_parts.push(format!("ISABOUT("));
                for (word_or_phrase_expr, weight_expr) in parameter {
                    let word_or_phrase = self.generate_expression(word_or_phrase_expr)?;
                    let weight = self.generate_expression(weight_expr)?;
                    sql_parts.push(format!("{} WEIGHT({})", word_or_phrase, weight));
                    sql_parts.push(String::from(", "));
                }
                sql_parts.remove(sql_parts.len() - 1);
                sql_parts.push(String::from(")"));
                sql_parts.join("")
            }
            _ => return Err(GenerateError::UnexpectedStatement(self.current.clone())),
        };
        self.write();
        Ok(sql)
    }

    // Generate expression, any search criteria or number or combination of those
    // Input: expression to generate
    // Output: string
    fn generate_expression(&mut self, expression: Expression) -> Result<String, GenerateError> {
        let sql: String = match expression {
            Expression::WordOrPhrase(s) => s,
            Expression::Number(u) => u.to_string(),
            Expression::ZeroToOne(f) => f.to_string(),
            // Infix operator enclose their expressions with parentheses to ensure precedence
            Expression::Infix(expr1, operator, expr2) => {
                let mut sql_parts = [
                    String::from("("),
                    self.generate_expression(*expr1)?,
                    String::from(")"),
                    self.generate_operator(operator)?,
                    String::from("("),
                    self.generate_expression(*expr2.clone())?,
                    String::from(")"),
                ];
                // If the second expression is a not operator it must write NOT before the parentheses
                match *expr2 {
                    Expression::Prefix(Operator::Not, ..) => sql_parts[4] = String::from("NOT ("),
                    _ => (),
                }
                sql_parts.join(" ")
            }
            Expression::Prefix(operator, expr) => {
                let sql_parts = [
                    self.generate_operator(operator)?,
                    self.generate_expression(*expr)?,
                ];
                sql_parts.join(" ")
            }
        };
        Ok(sql)
    }

    // Generate operator
    // Input: operator to generate
    // Output: string
    fn generate_operator(&mut self, operator: Operator) -> Result<String, GenerateError> {
        let op = match operator {
            Operator::And => "AND",
            Operator::Or => "OR",
            // has to be set infront of parentheses, see generate_expression for infix
            Operator::Not => "",
        };
        Ok(op.to_owned())
    }
}

// Types of error covered by the generator
#[derive(Debug, Error)]
pub enum GenerateError {
    #[error("Unexpected statement {0:?}.")]
    UnexpectedStatement(Statement),
}
