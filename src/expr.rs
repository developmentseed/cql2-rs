use crate::{Error, Geometry, SqlQuery, Validator};
use derive_is_enum_variant::is_enum_variant;
use json_dotpath::DotPaths;
use pg_escape::{quote_identifier, quote_literal};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::str::FromStr;

/// A CQL2 expression.
///
/// # Examples
///
/// [Expr] implements [FromStr]:
///
/// ```
/// use cql2::Expr;
///
/// let expr: Expr = "landsat:scene_id = 'LC82030282019133LGN00'".parse().unwrap();
/// ```
///
/// Use [Expr::to_text], [Expr::to_json], and [Expr::to_sql] to use the CQL2,
/// and use [Expr::is_valid] to check validity.
#[derive(Debug, Serialize, Deserialize, Clone, is_enum_variant)]
#[serde(untagged)]
#[allow(missing_docs)]
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
    Geometry(Geometry),
}

impl From<Value> for Expr {
    fn from(v: Value) -> Expr {
        let e: Expr = serde_json::from_value(v).unwrap();
        e
    }
}

impl From<Expr> for Value {
    fn from(v: Expr) -> Value {
        let v: Value = serde_json::to_value(v).unwrap();
        v
    }
}


impl TryInto<f64> for Expr {
    type Error = ();
    fn try_into(self) -> Result<f64, Self::Error> {
        match self {
            Expr::Float(v) => Ok(v),
            Expr::Literal(v) => f64::from_str(&v).or(Err(())),
            _ => Err(()),
        }
    }
}

impl TryInto<bool> for Expr {
    type Error = ();
    fn try_into(self) -> Result<bool, Self::Error> {
        match self {
            Expr::Bool(v) => Ok(v),
            _ => Err(()),
        }
    }
}

impl Expr {
    /// Insert values from properties from json
    ///
    ///  # Examples
    ///
    /// ```
    /// use serde_json::{json, Value};
    /// use cql2::Expr;
    /// let item = json!({"properties":{"eo:cloud_cover":10, "datetime": "2020-01-01 00:00:00Z", "boolfield": true}});
    /// let mut expr_json = json!(
    ///     {
    ///         "op": "+",
    ///         "args": [
    ///             {"property": "eo:cloud_cover"},
    ///             10
    ///         ]
    ///     }
    /// );
    ///
    /// let mut expr: Expr = serde_json::from_value(expr_json).unwrap();
    /// println!("Initial {:?}", expr);
    /// expr.reduce(&item);
    ///
    /// let output: f64;
    /// if let Expr::Float(v) = expr {
    ///     output = v;
    /// } else {
    ///     assert!(false);
    ///     output = 0.0;
    /// }
    /// println!("Modified {:?}", expr);
    ///
    /// assert_eq!(20.0, output);
    ///
    ///
    /// ```
    pub fn reduce(&mut self, j: &Value) {
        match self {
            Expr::Property { property } => {
                let propexpr = j.dot_get(property).or_else(|_| j.dot_get(&format!("properties.{}", property)))?;
                if let Some(v) = propexpr {
                    *self = Expr::from(v);
                }
            }
            Expr::Operation { op, args } => {
                let mut alltrue: bool = true;
                let mut anytrue: bool = false;
                let mut allbool: bool = true;
                for arg in args.iter_mut() {
                    arg.reduce(j);
                    let b: Result<bool, _> = arg.as_ref().clone().try_into();
                    match b {
                        Ok(true) => anytrue = true,
                        Ok(false) => {
                            alltrue = false;
                        }
                        _ => {
                            alltrue = false;
                            allbool = false;
                        }
                    }
                }

                // boolean operators
                if allbool {
                    match op.as_str() {
                        "and" => {
                            *self = Expr::Bool(alltrue);
                        }
                        "or" => {
                            *self = Expr::Bool(anytrue);
                        }
                        _ => (),
                    }
                    return;
                }

                // binary operations
                if args.len() == 2 {
                    // numerical binary operations
                    let left: Result<f64, ()> = (*args[0].clone()).try_into();
                    let right: Result<f64, ()> = (*args[1].clone()).try_into();
                    if let (Ok(l), Ok(r)) = (left, right) {
                        match op.as_str() {
                            "+" => {
                                *self = Expr::Float(l + r);
                            }
                            "-" => {
                                *self = Expr::Float(l - r);
                            }
                            "*" => {
                                *self = Expr::Float(l * r);
                            }
                            "/" => {
                                *self = Expr::Float(l / r);
                            }
                            "%" => {
                                *self = Expr::Float(l % r);
                            }
                            "^" => {
                                *self = Expr::Float(l.powf(r));
                            }
                            "=" => {
                                *self = Expr::Bool(l == r);
                            }
                            "<=" => {
                                *self = Expr::Bool(l <= r);
                            }
                            "<" => {
                                *self = Expr::Bool(l < r);
                            }
                            ">=" => {
                                *self = Expr::Bool(l >= r);
                            }
                            ">" => {
                                *self = Expr::Bool(l > r);
                            }
                            "<>" => {
                                *self = Expr::Bool(l != r);
                            }
                            _ => (),
                        }
                    }
                }
            }
            _ => (),
        }
    }
    /// Run CQL against a JSON Value
    ///
    ///  # Examples
    ///
    /// ```
    /// use serde_json::{json, Value};
    /// use cql2::Expr;
    /// let item = json!({"properties":{"eo:cloud_cover":10, "datetime": "2020-01-01 00:00:00Z", "boolfield": true}});
    /// let mut expr_json = json!(
    ///     {
    ///         "op" : ">",
    ///         "args" : [
    ///             {
    ///                  "op": "+",
    ///                  "args": [
    ///                     {"property": "eo:cloud_cover"},
    ///                     17
    ///                  ]
    ///             },
    ///             2
    ///         ]
    ///     }
    /// );
    ///
    ///
    /// let mut expr: Expr = serde_json::from_value(expr_json).unwrap();
    ///
    ///
    /// assert_eq!(true, expr.matches(&item).unwrap());
    ///
    ///
    /// let mut expr2: Expr = "boolfield and 1 + 2 = 3".parse().unwrap();
    /// assert_eq!(true, expr2.matches(&item).unwrap());
    /// ```
    pub fn matches(&self, j: &Value) -> Result<bool, ()> {
        let mut e = self.clone();
        e.reduce(j);
        match e {
            Expr::Bool(v) => Ok(v),
            _ => Err(()),
        }
    }
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
            Expr::Literal(v) => Ok(quote_literal(v).to_string()),
            Expr::Property { property } => Ok(quote_identifier(property).to_string()),
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
            Expr::Geometry(v) => v.to_wkt(),
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
                    "like" => Ok(format!("({} LIKE {})", a[0], a[1])),
                    "in" => Ok(format!("({} IN {})", a[0], a[1])),
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
                    "isNull" => {
                        check_len!("is null", a, 1, format!("({} IS NULL)", a[0]))
                    }
                    "+" | "-" | "*" | "/" | "%" => {
                        let paddedop = format!(" {} ", op);
                        Ok(a.join(&paddedop).to_string())
                    }
                    "^" | "=" | "<=" | "<" | "<>" | ">" | ">=" => {
                        check_len!(op, a, 2, format!("({} {} {})", a[0], op, a[1]))
                    }
                    _ => Ok(format!("{}({})", quote_identifier(op), a.join(", "))),
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
    pub fn to_sql(&self) -> Result<SqlQuery, Error> {
        let params: &mut Vec<String> = &mut vec![];
        let query = self.to_sql_inner(params)?;
        Ok(SqlQuery {
            query,
            params: params.to_vec(),
        })
    }

    fn to_sql_inner(&self, params: &mut Vec<String>) -> Result<String, Error> {
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
                params.push(format!("EPSG:4326;{}", v.to_wkt()?));
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
    pub fn to_json(&self) -> Result<String, Error> {
        serde_json::to_string(&self).map_err(Error::from)
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
    pub fn to_json_pretty(&self) -> Result<String, Error> {
        serde_json::to_string_pretty(&self).map_err(Error::from)
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
    pub fn to_value(&self) -> Result<Value, Error> {
        serde_json::to_value(self).map_err(Error::from)
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
#[cfg(test)]
mod tests {
    use super::Expr;

    #[test]
    fn keep_z() {
        let point: Expr = "POINT Z(-105.1019 40.1672 4981)".parse().unwrap();
        assert_eq!("POINT Z(-105.1019 40.1672 4981)", point.to_text().unwrap());
    }

    #[test]
    fn implicit_z() {
        let point: Expr = "POINT (-105.1019 40.1672 4981)".parse().unwrap();
        assert_eq!("POINT Z(-105.1019 40.1672 4981)", point.to_text().unwrap());
    }

    #[test]
    fn keep_m() {
        let point: Expr = "POINT M(-105.1019 40.1672 42)".parse().unwrap();
        assert_eq!("POINT M(-105.1019 40.1672 42)", point.to_text().unwrap());
    }

    #[test]
    fn keep_zm() {
        let point: Expr = "POINT ZM(-105.1019 40.1672 4981 42)".parse().unwrap();
        assert_eq!(
            "POINT ZM(-105.1019 40.1672 4981 42)",
            point.to_text().unwrap()
        );
    }
}
