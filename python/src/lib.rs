use pyo3::{
    exceptions::{PyException, PyIOError, PyValueError},
    prelude::*,
};
use std::path::PathBuf;

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

#[pymethods]
impl Expr {
    /// Parses a CQL2 expression from a file path.
    #[staticmethod]
    fn from_path(path: PathBuf) -> PyResult<Expr> {
        ::cql2::parse_file(path).map(Expr).map_err(to_py_error)
    }

    /// Parses a CQL2 expression.
    #[new]
    fn new(cql2: Bound<'_, PyAny>) -> PyResult<Self> {
        if let Ok(s) = cql2.extract::<&str>() {
            s.parse().map(Expr).map_err(to_py_error)
        } else {
            let expr: ::cql2::Expr = pythonize::depythonize(&cql2)?;
            Ok(Expr(expr))
        }
    }

    /// Converts this expression to a cql2-json dictionary.
    fn to_json<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        pythonize::pythonize(py, &self.0).map_err(PyErr::from)
    }

    /// Converts this expression to cql2-text.
    fn to_text<'py>(&self) -> PyResult<String> {
        self.0.to_text().map_err(to_py_error)
    }

    /// Converts this expression to SQL query.
    fn to_sql(&self) -> PyResult<SqlQuery> {
        self.0.to_sql().map(SqlQuery::from).map_err(to_py_error)
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

fn to_py_error(error: ::cql2::Error) -> PyErr {
    use ::cql2::Error::*;
    match error {
        InvalidCql2Text(..)
        | InvalidNumberOfArguments { .. }
        | MissingArgument(..)
        | ParseBool(..)
        | ParseFloat(..)
        | ParseInt(..) => PyValueError::new_err(error.to_string()),
        Io(err) => PyIOError::new_err(err.to_string()),
        _ => PyException::new_err(error.to_string()),
    }
}

/// A Python module implemented in Rust.
#[pymodule]
fn cql2(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<Expr>()?;
    m.add_class::<SqlQuery>()?;
    Ok(())
}
