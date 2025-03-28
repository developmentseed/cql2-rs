use crate::{geometry::spatial_op, temporal::temporal_op, Error, Geometry, SqlQuery, Validator};
use geo_types::Geometry as GGeom;
use geo_types::{coord, Rect};
use json_dotpath::DotPaths;
use like::Like;
use pg_escape::{quote_identifier, quote_literal};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashSet;
use std::fmt::Debug;
use std::ops::{Add, Deref};
use std::str::FromStr;
use unaccent::unaccent;
use wkt::TryFromWkt;

/// Boolean Operators
pub const BOOLOPS: &[&str] = &["and", "or"];

/// Equality Operators
pub const EQOPS: &[&str] = &["=", "<>"];

/// Comparison Operators
pub const CMPOPS: &[&str] = &[">", ">=", "<", "<="];

/// Spatial Operators
pub const SPATIALOPS: &[&str] = &[
    "s_equals",
    "s_intersects",
    "s_disjoint",
    "s_touches",
    "s_within",
    "s_overlaps",
    "s_crosses",
    "s_contains",
];

/// Temporal Operators
pub const TEMPORALOPS: &[&str] = &[
    "t_before",
    "t_after",
    "t_meets",
    "t_metby",
    "t_overlaps",
    "t_overlappedby",
    "t_starts",
    "t_startedby",
    "t_during",
    "t_contains",
    "t_finishes",
    "to_finishedby",
    "t_equals",
    "t_disjoint",
    "t_intersects",
];

/// Arithmetic Operators
pub const ARITHOPS: &[&str] = &["+", "-", "*", "/", "%", "^", "div"];

/// Array Operators
pub const ARRAYOPS: &[&str] = &["a_equals", "a_contains", "a_containedby", "a_overlaps"];

// todo: array ops, in, casei, accenti, between, not, like

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
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, PartialOrd)]
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

impl TryFrom<Value> for Expr {
    type Error = Error;
    fn try_from(v: Value) -> Result<Expr, Error> {
        serde_json::from_value(v).map_err(Error::from)
    }
}

impl TryFrom<Expr> for Value {
    type Error = Error;
    fn try_from(v: Expr) -> Result<Value, Error> {
        serde_json::to_value(v).map_err(Error::from)
    }
}

impl TryFrom<Expr> for f64 {
    type Error = Error;
    fn try_from(v: Expr) -> Result<f64, Error> {
        match v {
            Expr::Float(v) => Ok(v),
            Expr::Literal(v) => f64::from_str(&v).map_err(Error::from),
            _ => Err(Error::ExprToF64(v)),
        }
    }
}

impl TryFrom<&Expr> for bool {
    type Error = Error;
    fn try_from(v: &Expr) -> Result<bool, Error> {
        match v {
            Expr::Bool(v) => Ok(*v),
            Expr::Literal(v) => bool::from_str(v).map_err(Error::from),
            _ => Err(Error::ExprToBool(v.clone())),
        }
    }
}

impl TryFrom<Expr> for String {
    type Error = Error;
    fn try_from(v: Expr) -> Result<String, Error> {
        match v {
            Expr::Literal(v) => Ok(v),
            Expr::Bool(v) => Ok(v.to_string()),
            Expr::Float(v) => Ok(v.to_string()),
            _ => Err(Error::ExprToBool(v)),
        }
    }
}

impl TryFrom<Expr> for GGeom {
    type Error = Error;
    fn try_from(v: Expr) -> Result<GGeom, Error> {
        match v {
            Expr::Geometry(v) => Ok(GGeom::try_from_wkt_str(&v.to_wkt().unwrap())
                .expect("Failed to convert WKT to Geometry")),
            Expr::BBox { ref bbox } => {
                let minx: f64 = bbox[0].as_ref().clone().try_into()?;
                let miny: f64 = bbox[1].as_ref().clone().try_into()?;
                let maxx: f64;
                let maxy: f64;

                match bbox.len() {
                    4 => {
                        maxx = bbox[2].as_ref().clone().try_into()?;
                        maxy = bbox[3].as_ref().clone().try_into()?;
                    }
                    6 => {
                        maxx = bbox[3].as_ref().clone().try_into()?;
                        maxy = bbox[4].as_ref().clone().try_into()?;
                    }
                    _ => return Err(Error::ExprToGeom(v.clone())),
                };
                let rec = Rect::new(coord! {x:minx, y:miny}, coord! {x:maxx,y:maxy});
                Ok(rec.into())
            }
            _ => Err(Error::ExprToGeom(v)),
        }
    }
}

impl TryFrom<Expr> for HashSet<String> {
    type Error = Error;
    fn try_from(v: Expr) -> Result<HashSet<String>, Error> {
        match v {
            Expr::Array(v) => {
                let mut h = HashSet::new();
                for el in v {
                    let _ = h.insert(el.to_text()?);
                }
                Ok(h)
            }
            _ => Err(Error::ExprToGeom(v)),
        }
    }
}

fn cmp_op<T: PartialEq + PartialOrd>(left: T, right: T, op: &str) -> Result<Expr, Error> {
    let out = match op {
        "=" => Ok(left == right),
        "<=" => Ok(left <= right),
        "<" => Ok(left < right),
        ">=" => Ok(left >= right),
        ">" => Ok(left > right),
        "<>" => Ok(left != right),
        _ => Err(Error::OpNotImplemented("Binary Bool")),
    };
    match out {
        Ok(v) => Ok(Expr::Bool(v)),
        _ => Err(Error::OperationError()),
    }
}

fn arith_op(left: Expr, right: Expr, op: &str) -> Result<Expr, Error> {
    let left = f64::try_from(left)?;
    let right = f64::try_from(right)?;
    let out = match op {
        "+" => Ok(left + right),
        "-" => Ok(left - right),
        "*" => Ok(left * right),
        "/" => Ok(left / right),
        "%" => Ok(left % right),
        "^" => Ok(left.powf(right)),
        _ => Err(Error::OpNotImplemented("Arith")),
    };
    match out {
        Ok(v) => Ok(Expr::Float(v)),
        _ => Err(Error::OperationError()),
    }
}

fn array_op(left: Expr, right: Expr, op: &str) -> Result<Expr, Error> {
    let left: HashSet<String> = left.try_into()?;
    let right: HashSet<String> = right.try_into()?;
    let out = match op {
        "a_equals" => Ok(left == right),
        "a_contains" => Ok(left.is_superset(&right)),
        "a_containedby" => Ok(left.is_subset(&right)),
        "a_overlaps" => Ok(!left.is_disjoint(&right)),
        _ => Err(Error::OpNotImplemented("Arith")),
    };
    match out {
        Ok(v) => Ok(Expr::Bool(v)),
        _ => Err(Error::OperationError()),
    }
}

impl Expr {
    /// Update this expression with values from the `properties` attribute of a JSON object
    ///
    ///  # Examples
    ///
    /// ```
    /// use serde_json::{json, Value};
    /// use cql2::Expr;
    /// use std::str::FromStr;
    ///
    /// let item = json!({"properties":{"eo:cloud_cover":10, "datetime": "2020-01-01 00:00:00Z", "boolfield": true}});
    ///
    /// let fromexpr: Expr = Expr::from_str("boolfield = true").unwrap();
    /// let reduced = fromexpr.reduce(Some(&item)).unwrap();
    /// let toexpr: Expr = Expr::from_str("true").unwrap();
    /// assert_eq!(reduced, toexpr);
    ///
    /// let fromexpr: Expr = Expr::from_str("\"eo:cloud_cover\" + 10").unwrap();
    /// let reduced = fromexpr.reduce(Some(&item)).unwrap();
    /// let toexpr: Expr = Expr::from_str("20").unwrap();
    /// assert_eq!(reduced, toexpr);
    ///
    /// let fromexpr: Expr = Expr::from_str("(bork=1) and (bork=1) and (bork=1 and true)").unwrap();
    /// let reduced = fromexpr.reduce(Some(&item)).unwrap();
    /// let toexpr: Expr = Expr::from_str("bork=1").unwrap();
    /// assert_eq!(reduced, toexpr);
    ///
    /// ```
    pub fn reduce(self, j: Option<&Value>) -> Result<Expr, Error> {
        match self {
            Expr::Property { ref property } => {
                if let Some(j) = j {
                    if let Some(value) = j.dot_get::<Value>(property)? {
                        Expr::try_from(value)
                    } else if let Some(value) =
                        j.dot_get::<Value>(&format!("properties.{}", property))?
                    {
                        Expr::try_from(value)
                    } else {
                        Ok(self)
                    }
                } else {
                    Ok(self)
                }
            }
            Expr::Operation { op, args } => {
                let args: Vec<Box<Expr>> = args
                    .into_iter()
                    .map(|expr| expr.reduce(j).map(Box::new))
                    .collect::<Result<_, _>>()?;

                if BOOLOPS.contains(&op.as_str()) {
                    let curop = op.clone();
                    let mut dedupargs: Vec<Box<Expr>> = vec![];
                    let mut nestedargs: Vec<Box<Expr>> = vec![];
                    for a in args {
                        match *a {
                            Expr::Operation { op, args } if op == curop => {
                                nestedargs.append(&mut args.clone());
                            }
                            _ => {
                                dedupargs.push(a.clone());
                            }
                        }
                    }
                    dedupargs.append(&mut nestedargs);
                    dedupargs.sort_by(|a, b| a.partial_cmp(b).unwrap());
                    dedupargs.dedup();

                    let mut anytrue: bool = false;
                    let mut anyfalse: bool = false;
                    let mut anyexp: bool = false;

                    for a in dedupargs.iter() {
                        let b = bool::try_from(a.as_ref());
                        match b {
                            Ok(true) => {
                                anytrue = true;
                            }
                            Ok(false) => {
                                anyfalse = true;
                            }
                            _ => {
                                anyexp = true;
                            }
                        }
                    }
                    if op == "and" && anytrue {
                        dedupargs.retain(|x| !bool::try_from(x.as_ref()).unwrap_or(false));
                    }
                    if dedupargs.len() == 1 {
                        Ok(dedupargs.pop().unwrap().as_ref().clone())
                    } else if (op == "and" && anyfalse) || (op == "or" && !anytrue && !anyexp) {
                        Ok(Expr::Bool(false))
                    } else if (op == "and" && !anyfalse && !anyexp) || (op == "or" && anytrue) {
                        Ok(Expr::Bool(true))
                    } else {
                        Ok(Expr::Operation {
                            op,
                            args: dedupargs.clone(),
                        })
                    }
                } else if op == "not" {
                    match args[0].deref() {
                        Expr::Bool(v) => Ok(Expr::Bool(!v)),
                        _ => Ok(Expr::Operation { op, args }),
                    }
                } else if op == "casei" {
                    match args[0].as_ref() {
                        Expr::Literal(v) => Ok(Expr::Literal(v.to_lowercase())),
                        _ => Ok(Expr::Operation { op, args }),
                    }
                } else if op == "accenti" {
                    match args[0].as_ref() {
                        Expr::Literal(v) => Ok(Expr::Literal(unaccent(v))),
                        _ => Ok(Expr::Operation { op, args }),
                    }
                } else if op == "between" {
                    Ok(Expr::Bool(args[0] >= args[1] && args[0] <= args[2]))
                } else if args.len() != 2 {
                    Ok(Expr::Operation { op, args })
                } else {
                    // Two-arg operations
                    let left = args[0].deref().clone();
                    let right = args[1].deref().clone();

                    if std::mem::discriminant(&left) == std::mem::discriminant(&right) {
                        if SPATIALOPS.contains(&op.as_str()) {
                            Ok(spatial_op(left, right, &op)
                                .unwrap_or_else(|_| Expr::Operation { op, args }))
                        } else if TEMPORALOPS.contains(&op.as_str()) {
                            Ok(temporal_op(left, right, &op)
                                .unwrap_or_else(|_| Expr::Operation { op, args }))
                        } else if ARITHOPS.contains(&op.as_str()) {
                            Ok(arith_op(left, right, &op)
                                .unwrap_or_else(|_| Expr::Operation { op, args }))
                        } else if EQOPS.contains(&op.as_str()) || CMPOPS.contains(&op.as_str()) {
                            Ok(cmp_op(left, right, &op)
                                .unwrap_or_else(|_| Expr::Operation { op, args }))
                        } else if ARRAYOPS.contains(&op.as_str()) {
                            Ok(array_op(left, right, &op)
                                .unwrap_or_else(|_| Expr::Operation { op, args }))
                        } else if op == "like" {
                            let l: String = left.try_into()?;
                            let r: String = right.try_into()?;
                            let m: bool = Like::<true>::like(l.as_str(), r.as_str())?;
                            Ok(Expr::Bool(m))
                        } else {
                            Ok(Expr::Operation { op, args })
                        }
                    } else if op == "in" {
                        let l: String = left.to_text()?;
                        let r: HashSet<String> = right.try_into()?;
                        let isin: bool = r.contains(&l);
                        Ok(Expr::Bool(isin))
                    } else {
                        Ok(Expr::Operation { op, args })
                    }
                }
            }
            _ => Ok(self),
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
    ///
    /// let mut expr: Expr = "boolfield and 1 + 2 = 3".parse().unwrap();
    /// assert_eq!(true, expr.matches(Some(&item)).unwrap());
    ///
    /// let mut expr: Expr = "eo:cloud_cover <= 9".parse().unwrap();
    /// assert_eq!(false, expr.matches(Some(&item)).unwrap());
    /// ```
    pub fn matches(self, j: Option<&Value>) -> Result<bool, Error> {
        let reduced = self.reduce(j)?;
        match reduced {
            Expr::Bool(v) => Ok(v),
            _ => Err(Error::NonReduced()),
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
                validator.is_valid(value)
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

impl Add for Expr {
    type Output = Expr;

    ///
    /// Combines two expressions with the `+` operator.
    ///
    /// # Examples
    ///
    /// ```
    /// use cql2::Expr;
    /// use std::ops::Add;
    ///
    /// let expr1 = Expr::Bool(true);
    /// let expr2 = Expr::Bool(false);
    /// let expected_expr: Expr = "true and false".parse().unwrap();
    /// assert_eq!(expr1 + expr2, expected_expr);
    /// ```
    ///
    /// ```
    /// use cql2::Expr;
    /// use std::ops::Add;
    ///
    /// let expr1 = Expr::Bool(true);
    /// let expr2 = Expr::Bool(false);
    /// let expected_expr: Expr = "true and false".parse().unwrap();
    /// assert_eq!(expr1.add(expr2), expected_expr);
    /// ```
    fn add(self, other: Expr) -> Expr {
        Expr::Operation {
            op: "and".to_string(),
            args: vec![Box::new(self), Box::new(other)],
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
