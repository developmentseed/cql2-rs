use crate::{Error, SqlQuery, Validator};
use geozero::{geojson::GeoJsonString, ToWkt};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::str::FromStr;

/// A CQL2 expression.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum Expr {
    Operation { op: String, args: Vec<Box<Expr>> },
    Interval { interval: Vec<Box<Expr>> },
    Timestamp { timestamp: Box<Expr> },
    Date { date: Box<Expr> },
    Property { property: String },
    BBox { bbox: Vec<Box<Expr>> },
    Float(f64),
    Literal(String),
    Bool(bool),
    Array(Vec<Box<Expr>>),
    Geometry(serde_json::Value),
}

impl Expr {
    /// Converts this expression to CQL2 text.
    ///
    /// # Examples
    ///
    /// ```
    /// use cql2::Expr;
    ///
    /// let expr = Expr::Bool(true);
    /// assert_eq!(expr.to_text().unwrap(), "true");
    /// ```
    pub fn to_text(&self) -> Result<String, Error> {
        macro_rules! check_len {
            ($name:expr, $args:expr, $len:expr, $text:expr) => {
                if $args.len() == $len {
                    Ok($text)
                } else {
                    Err(Error::InvalidNumberOfArguments {
                        name: $name.to_string(),
                        actual: $args.len(),
                        expected: $len,
                    })
                }
            };
        }

        match self {
            Expr::Bool(v) => Ok(v.to_string()),
            Expr::Float(v) => Ok(v.to_string()),
            Expr::Literal(v) => Ok(format!("'{}'", v)),
            Expr::Property { property } => Ok(format!("\"{property}\"")),
            Expr::Interval { interval } => {
                check_len!(
                    "interval",
                    interval,
                    2,
                    format!(
                        "INTERVAL({},{})",
                        interval[0].to_text()?,
                        interval[1].to_text()?
                    )
                )
            }
            Expr::Date { date } => Ok(format!("DATE({})", date.to_text()?)),
            Expr::Timestamp { timestamp } => Ok(format!("TIMESTAMP({})", timestamp.to_text()?)),
            Expr::Geometry(v) => {
                let gj = GeoJsonString(v.to_string());
                gj.to_wkt().map_err(Error::from)
            }
            Expr::Array(v) => {
                let array_els: Vec<String> =
                    v.iter().map(|a| a.to_text()).collect::<Result<_, _>>()?;
                Ok(format!("({})", array_els.join(", ")))
            }
            Expr::Operation { op, args } => {
                let a: Vec<String> = args.iter().map(|x| x.to_text()).collect::<Result<_, _>>()?;
                match op.as_str() {
                    "and" => Ok(format!("({})", a.join(" AND "))),
                    "or" => Ok(format!("({})", a.join(" OR "))),
                    "between" => {
                        check_len!(
                            "between",
                            a,
                            3,
                            format!("({} BETWEEN {} AND {})", a[0], a[1], a[2])
                        )
                    }
                    "not" => {
                        check_len!("not", a, 1, format!("(NOT {})", a[0]))
                    }
                    "is null" => {
                        check_len!("is null", a, 1, format!("({} IS NULL)", a[0]))
                    }
                    "+" | "-" | "*" | "/" | "%" | "^" | "=" | "<=" | "<" | "<>" | ">" | ">=" => {
                        check_len!(op, a, 2, format!("({} {} {})", a[0], op, a[1]))
                    }
                    _ => Ok(format!("{}({})", op, a.join(", "))),
                }
            }
            Expr::BBox { bbox } => {
                let array_els: Vec<String> =
                    bbox.iter().map(|a| a.to_text()).collect::<Result<_, _>>()?;
                Ok(format!("BBOX({})", array_els.join(", ")))
            }
        }
    }

    /// Converts this expression to a [SqlQuery] struct with parameters
    /// separated to use with parameter binding.
    ///
    /// # Examples
    ///
    /// ```
    /// use cql2::Expr;
    ///
    /// let expr = Expr::Bool(true);
    /// let s = expr.to_sql().unwrap();
    /// ```
    pub fn to_sql(&self) -> Result<SqlQuery, geozero::error::GeozeroError> {
        let params: &mut Vec<String> = &mut vec![];
        let query = self.to_sql_inner(params)?;
        Ok(SqlQuery {
            query,
            params: params.to_vec(),
        })
    }

    fn to_sql_inner(
        &self,
        params: &mut Vec<String>,
    ) -> Result<String, geozero::error::GeozeroError> {
        Ok(match self {
            Expr::Bool(v) => {
                params.push(v.to_string());
                format!("${}", params.len())
            }
            Expr::Float(v) => {
                params.push(v.to_string());
                format!("${}", params.len())
            }
            Expr::Literal(v) => {
                params.push(v.to_string());
                format!("${}", params.len())
            }
            Expr::Date { date } => date.to_sql_inner(params)?,
            Expr::Timestamp { timestamp } => timestamp.to_sql_inner(params)?,

            Expr::Interval { interval } => {
                let a: Vec<String> = interval
                    .iter()
                    .map(|x| x.to_sql_inner(params))
                    .collect::<Result<_, _>>()?;
                format!("TSTZRANGE({},{})", a[0], a[1],)
            }
            Expr::Geometry(v) => {
                let gj = GeoJsonString(v.to_string());
                params.push(format!("EPSG:4326;{}", gj.to_wkt()?));
                format!("${}", params.len())
            }
            Expr::Array(v) => {
                let array_els: Vec<String> = v
                    .iter()
                    .map(|a| a.to_sql_inner(params))
                    .collect::<Result<_, _>>()?;
                format!("[{}]", array_els.join(", "))
            }
            Expr::Property { property } => format!("\"{property}\""),
            Expr::Operation { op, args } => {
                let a: Vec<String> = args
                    .iter()
                    .map(|x| x.to_sql_inner(params))
                    .collect::<Result<_, _>>()?;
                match op.as_str() {
                    "and" => format!("({})", a.join(" AND ")),
                    "or" => format!("({})", a.join(" OR ")),
                    "between" => format!("({} BETWEEN {} AND {})", a[0], a[1], a[2]),
                    "not" => format!("(NOT {})", a[0]),
                    "is null" => format!("({} IS NULL)", a[0]),
                    "+" | "-" | "*" | "/" | "%" | "^" | "=" | "<=" | "<" | "<>" | ">" | ">=" => {
                        format!("({} {} {})", a[0], op, a[1])
                    }
                    _ => format!("{}({})", op, a.join(", ")),
                }
            }
            Expr::BBox { bbox } => {
                let array_els: Vec<String> = bbox
                    .iter()
                    .map(|a| a.to_sql_inner(params))
                    .collect::<Result<_, _>>()?;
                format!("[{}]", array_els.join(", "))
            }
        })
    }

    /// Converts this expression to a JSON string.
    ///
    /// # Examples
    ///
    /// ```
    /// use cql2::Expr;
    ///
    /// let expr = Expr::Bool(true);
    /// let s = expr.to_json().unwrap();
    /// ```
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(&self)
    }

    /// Converts this expression to a pretty JSON string.
    ///
    /// # Examples
    ///
    /// ```
    /// use cql2::Expr;
    ///
    /// let expr = Expr::Bool(true);
    /// let s = expr.to_json_pretty().unwrap();
    /// ```
    pub fn to_json_pretty(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(&self)
    }

    /// Converts this expression to a [serde_json::Value].
    ///
    /// # Examples
    ///
    /// ```
    /// use cql2::Expr;
    ///
    /// let expr = Expr::Bool(true);
    /// let value = expr.to_value().unwrap();
    /// ```
    pub fn to_value(&self) -> Result<Value, serde_json::Error> {
        serde_json::to_value(self)
    }

    /// Returns true if this expression is valid CQL2.
    ///
    /// For detailed error reporting, use [Validator::validate] in conjunction with [Expr::to_value].
    ///
    /// # Examples
    ///
    /// ```
    /// use cql2::Expr;
    ///
    /// let expr = Expr::Bool(true);
    /// assert!(expr.is_valid());
    /// ```
    ///
    /// # Panics
    ///
    /// Panics if the default validator can't be created.
    pub fn is_valid(&self) -> bool {
        let value = serde_json::to_value(self);
        match &value {
            Ok(value) => {
                let validator = Validator::new().expect("Could not create default validator");
                validator.validate(value).is_ok()
            }
            _ => false,
        }
    }
}

impl FromStr for Expr {
    type Err = Error;

    fn from_str(s: &str) -> Result<Expr, Error> {
        if s.starts_with('{') {
            crate::parse_json(s).map_err(Error::from)
        } else {
            crate::parse_text(s)
        }
    }
}
