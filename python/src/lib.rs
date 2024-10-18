#![allow(clippy::result_large_err)]

use pyo3::{
    create_exception,
    exceptions::{PyException, PyIOError, PyValueError},
    prelude::*,
};
use std::path::PathBuf;

create_exception!(cql2, ValidationError, PyException);
create_exception!(cql2, ParseError, PyException);

/// Crate-specific error enum.
#[allow(clippy::large_enum_variant)]
enum Error {
    Cql2(::cql2::Error),
    Pythonize(pythonize::PythonizeError),
}

/// Crate specific result type.
type Result<T> = std::result::Result<T, Error>;

/// A CQL2 expression.
#[pyclass]
struct Expr(::cql2::Expr);

/// A SQL query
#[pyclass]
struct SqlQuery {
    #[pyo3(get)]
    query: String,

    #[pyo3(get)]
    params: Vec<String>,
}

#[pyfunction]
fn parse_file(path: PathBuf) -> Result<Expr> {
    ::cql2::parse_file(path).map(Expr).map_err(Error::from)
}

#[pyfunction]
fn parse_json(s: &str) -> PyResult<Expr> {
    ::cql2::parse_json(s)
        .map(Expr)
        .map_err(|err| ParseError::new_err(err.to_string()))
}

#[pyfunction]
fn parse_text(s: &str) -> PyResult<Expr> {
    ::cql2::parse_text(s)
        .map(Expr)
        .map_err(|err| ParseError::new_err(err.to_string()))
}

#[pymethods]
impl Expr {
    #[new]
    fn new(cql2: Bound<'_, PyAny>) -> Result<Self> {
        if let Ok(s) = cql2.extract::<&str>() {
            s.parse().map(Expr).map_err(Error::from)
        } else {
            let expr: ::cql2::Expr = pythonize::depythonize(&cql2)?;
            Ok(Expr(expr))
        }
    }

    fn validate(&self) -> PyResult<()> {
        let validator = ::cql2::Validator::new().map_err(Error::from)?;
        if let Err(error) = validator.validate(&self.0.to_value().map_err(Error::from)?) {
            Err(ValidationError::new_err(error.to_string()))
        } else {
            Ok(())
        }
    }

    fn to_json<'py>(&self, py: Python<'py>) -> Result<Bound<'py, PyAny>> {
        pythonize::pythonize(py, &self.0).map_err(Error::from)
    }

    fn to_text(&self) -> Result<String> {
        self.0.to_text().map_err(Error::from)
    }

    fn to_sql(&self) -> Result<SqlQuery> {
        self.0.to_sql().map(SqlQuery::from).map_err(Error::from)
    }
}

impl From<::cql2::SqlQuery> for SqlQuery {
    fn from(value: ::cql2::SqlQuery) -> Self {
        SqlQuery {
            query: value.query,
            params: value.params,
        }
    }
}

impl From<Error> for PyErr {
    fn from(error: Error) -> PyErr {
        use ::cql2::Error::*;
        match error {
            Error::Cql2(error) => match error {
                InvalidCql2Text(..)
                | InvalidNumberOfArguments { .. }
                | MissingArgument(..)
                | ParseBool(..)
                | ParseFloat(..)
                | ParseInt(..) => PyValueError::new_err(error.to_string()),
                Io(err) => PyIOError::new_err(err.to_string()),
                _ => PyException::new_err(error.to_string()),
            },
            Error::Pythonize(error) => error.into(),
        }
    }
}

impl From<::cql2::Error> for Error {
    fn from(value: ::cql2::Error) -> Self {
        Error::Cql2(value)
    }
}

impl From<pythonize::PythonizeError> for Error {
    fn from(value: pythonize::PythonizeError) -> Self {
        Error::Pythonize(value)
    }
}

#[pyfunction]
fn main(py: Python<'_>) {
    use clap::Parser;

    let signal = py.import_bound("signal").unwrap();
    signal
        .getattr("signal")
        .unwrap()
        .call1((
            signal.getattr("SIGINT").unwrap(),
            signal.getattr("SIG_DFL").unwrap(),
        ))
        .unwrap();
    let args: Vec<_> = std::env::args().skip(1).collect();
    ::cql2_cli::Cli::parse_from(args).run()
}

/// A Python module implemented in Rust.
#[pymodule]
fn cql2(py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<Expr>()?;
    m.add_class::<SqlQuery>()?;
    m.add_function(wrap_pyfunction!(main, m)?)?;
    m.add_function(wrap_pyfunction!(parse_file, m)?)?;
    m.add_function(wrap_pyfunction!(parse_text, m)?)?;
    m.add_function(wrap_pyfunction!(parse_json, m)?)?;
    m.add("ParseError", py.get_type_bound::<ParseError>())?;
    m.add("ValidationError", py.get_type_bound::<ValidationError>())?;
    Ok(())
}
