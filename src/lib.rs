//! Parse and transform [Common Query Language 2 (CQL2)](https://docs.ogc.org/is/21-065r2/21-065r2.html).

#![deny(unused_crate_dependencies)]

mod error;
mod expr;
mod parser;
mod validator;

use serde_derive::{Deserialize, Serialize};
use std::{fs, io::Read, path::Path};
pub use {error::Error, expr::Expr, parser::parse_text, validator::Validator};

/// A SQL query, broken into the query and parameters.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SqlQuery {
    query: String,
    params: Vec<String>,
}

/// Parses a cql2-json string into a CQL2 expression.
///
/// # Examples
///
/// ```
/// let s = include_str!("../tests/fixtures/json/example01.json");
/// let expr = cql2::parse_json(s);
/// ```
pub fn parse_json(s: &str) -> Result<Expr, serde_json::Error> {
    serde_json::from_str(s)
}

/// Reads a file and returns its contents as a CQL2 expression;
///
/// # Examples
///
/// ```no_run
/// let expr = cql2::parse_file("tests/fixtures/json/example01.json");
/// ```
pub fn parse_file(path: impl AsRef<Path>) -> Result<Expr, Error> {
    let s = fs::read_to_string(path)?;
    s.parse()
}

fn get_stdin() -> Result<String, std::io::Error> {
    use std::{
        env,
        io::{self, IsTerminal},
    };
    let args: Vec<String> = env::args().collect();
    let mut buffer = String::new();

    if args.len() >= 2 {
        buffer = args[1].to_string();
    } else if io::stdin().is_terminal() {
        println!("Enter CQL2 as Text or JSON, then hit return");
        io::stdin().read_line(&mut buffer)?;
    } else {
        io::stdin().read_to_string(&mut buffer)?;
    }
    Ok(buffer)
}

fn parse_stderr(s: &str) -> Result<Expr, Error> {
    let debug_level: u8 = std::env::var("CQL2_DEBUG_LEVEL")
        .ok()
        .map(|s| s.parse())
        .transpose()?
        .unwrap_or(1);
    let validator = Validator::new().unwrap();

    let parsed: Expr = s.parse()?;
    let value = serde_json::to_value(&parsed)?;

    let validation = validator.validate(&value);

    match validation {
        Ok(()) => Ok(parsed),
        Err(err) => {
            eprintln!("Passed in CQL2 parsed to {value}.");
            eprintln!("This did not pass jsonschema validation for CQL2.");
            match debug_level {
                0 => eprintln!("For more detailed validation details set CQL2_DEBUG_LEVEL to 1."),
                1 => eprintln!(
                    "{err}\nFor more detailed validation details set CQL2_DEBUG_LEVEL to 2."
                ),
                2 => eprintln!(
                    "{err:#}\nFor more detailed validation details set CQL2_DEBUG_LEVEL to 3."
                ),
                _ => {
                    let detailed_output = err.detailed_output();
                    eprintln!("{detailed_output:#}");
                }
            }
            Err(Error::Validation(serde_json::to_value(
                err.detailed_output(),
            )?))
        }
    }
}

/// Parse standard input into a CQL2 expression.
///
/// # Examples
///
/// ```no_run
/// let expr = cql2::parse_stdin();
/// ```
pub fn parse_stdin() -> Result<Expr, Error> {
    let buffer = get_stdin()?;
    parse_stderr(&buffer)
}

#[cfg(test)]
use {assert_json_diff as _, rstest as _};
