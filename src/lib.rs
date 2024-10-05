//! Parse and transform [Common Query Language 2 (CQL2)](https://docs.ogc.org/is/21-065r2/21-065r2.html).

#![deny(unused_crate_dependencies)]

mod error;
mod expr;
mod parser;
mod validator;

use serde_derive::{Deserialize, Serialize};
use std::{fs, path::Path};
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

#[cfg(test)]
use {assert_json_diff as _, rstest as _};
