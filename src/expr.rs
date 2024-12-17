use crate::{
    geometry::spatial_op, temporal::temporal_op, DateRange, Error, Geometry, SqlQuery, Validator,
};
use enum_as_inner::EnumAsInner;
use geos::Geometry as GGeom;
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
#[derive(Debug, Serialize, Deserialize, Clone, EnumAsInner)]
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
            _ => Err(Error::ExprToF64()),
        }
    }
}

impl TryFrom<Expr> for bool {
    type Error = Error;
    fn try_from(v: Expr) -> Result<bool, Error> {
        match v {
            Expr::Bool(v) => Ok(v),
            Expr::Literal(v) => bool::from_str(&v).map_err(Error::from),
            _ => Err(Error::ExprToBool()),
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
            _ => Err(Error::ExprToBool()),
        }
    }
}

impl TryFrom<Expr> for GGeom {
    type Error = Error;
    fn try_from(v: Expr) -> Result<GGeom, Error> {
        match v {
            Expr::Geometry(v) => Ok(GGeom::new_from_wkt(&v.to_wkt().unwrap())
                .expect("Failed to convert WKT to Geos Geometry")),
            _ => Err(Error::ExprToGeom()),
        }
    }
}

impl PartialEq for Expr {
    fn eq(&self, other: &Self) -> bool {
        self.to_text().unwrap() == other.to_text().unwrap()
    }
}

fn binary_bool<T: PartialEq + PartialOrd>(left: &T, right: &T, op: &str) -> Result<bool, Error> {
    match op {
        "=" => Ok(left == right),
        "<=" => Ok(left <= right),
        "<" => Ok(left < right),
        ">=" => Ok(left >= right),
        ">" => Ok(left > right),
        "<>" => Ok(left != right),
        _ => Err(Error::OpNotImplemented("Binary Bool")),
    }
}

fn arith(left: &f64, right: &f64, op: &str) -> Result<f64, Error> {
    match op {
        "+" => Ok(left + right),
        "-" => Ok(left - right),
        "*" => Ok(left * right),
        "/" => Ok(left / right),
        "%" => Ok(left % right),
        "^" => Ok(left.powf(*right)),
        _ => Err(Error::OpNotImplemented("Arith")),
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
    /// let mut fromexpr: Expr = Expr::from_str("boolfield = true").unwrap();
    /// fromexpr.reduce(Some(&item));
    /// let mut toexpr: Expr = Expr::from_str("true").unwrap();
    /// assert_eq!(fromexpr, toexpr);
    ///
    /// let mut fromexpr: Expr = Expr::from_str("\"eo:cloud_cover\" + 10").unwrap();
    /// fromexpr.reduce(Some(&item));
    /// let mut toexpr: Expr = Expr::from_str("20").unwrap();
    /// assert_eq!(fromexpr, toexpr);
    ///
    /// ```
    pub fn reduce(&mut self, j: Option<&Value>) {
        match self {
            Expr::Interval { interval } => {
                for arg in interval.iter_mut() {
                    arg.reduce(j);
                }
            }
            Expr::Timestamp { timestamp } => {
                timestamp.reduce(j);
            }
            Expr::Date { date } => {
                date.reduce(j);
            }
            Expr::Property { property } => {
                if let Some(j) = j {
                    let propexpr: Option<Value>;
                    if j.dot_has(property) {
                        propexpr = j.dot_get(property).unwrap();
                    } else {
                        propexpr = j.dot_get(&format!("properties.{}", property)).unwrap();
                    }

                    println!("j:{:?} property:{:?}", j, property);
                    println!("propexpr: {:?}", propexpr);
                    if let Some(v) = propexpr {
                        *self = Expr::try_from(v).unwrap();
                    }
                }
            }
            Expr::Operation { op, args } => {
                let mut alltrue: bool = true;
                let mut anytrue: bool = false;
                let mut allbool: bool = true;
                for arg in args.iter_mut() {
                    arg.reduce(j);

                    if let Ok(bool) = arg.as_ref().clone().try_into() {
                        if bool {
                            anytrue = true;
                        } else {
                            alltrue = false;
                        }
                    } else {
                        alltrue = false;
                        allbool = false;
                    }
                }

                // boolean operators
                if allbool {
                    match op.as_str() {
                        "and" => {
                            *self = Expr::Bool(alltrue);
                            return;
                        }
                        "or" => {
                            *self = Expr::Bool(anytrue);
                            return;
                        }
                        _ => (),
                    }
                }

                // binary operations
                if args.len() == 2 {
                    let left: &Expr = args[0].as_ref();
                    let right: &Expr = args[1].as_ref();

                    if let (Ok(l), Ok(r)) =
                        (f64::try_from(left.clone()), f64::try_from(right.clone()))
                    {
                        if let Ok(v) = arith(&l, &r, op) {
                            *self = Expr::Float(v);
                            return;
                        }
                        if let Ok(v) = binary_bool(&l, &r, op) {
                            *self = Expr::Bool(v);
                            return;
                        }
                    } else if let (Ok(l), Ok(r)) =
                        (bool::try_from(left.clone()), bool::try_from(right.clone()))
                    {
                        if let Ok(v) = binary_bool(&l, &r, op) {
                            *self = Expr::Bool(v);
                            return;
                        }
                    } else if let (Ok(l), Ok(r)) = (
                        GGeom::try_from(left.clone()),
                        GGeom::try_from(right.clone()),
                    ) {
                        println!("Is Spatial Op. {:?} ({:?}, {:?})", op, left, right);
                        if let Ok(v) = spatial_op(&l, &r, op) {
                            *self = Expr::Bool(v);
                            return;
                        }
                    } else if let (Ok(l), Ok(r)) = (
                        DateRange::try_from(left.clone()),
                        DateRange::try_from(right.clone()),
                    ) {
                        if let Ok(v) = temporal_op(&l, &r, op) {
                            *self = Expr::Bool(v);
                            return;
                        }
                    } else if let (Ok(l), Ok(r)) = (
                        String::try_from(left.clone()),
                        String::try_from(right.clone()),
                    ) {
                        if let Ok(v) = binary_bool(&l, &r, op) {
                            *self = Expr::Bool(v);
                            return;
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
    ///
    /// let mut expr: Expr = "boolfield and 1 + 2 = 3".parse().unwrap();
    /// assert_eq!(true, expr.matches(Some(&item)).unwrap());
    /// ```
    pub fn matches(&self, j: Option<&Value>) -> Result<bool, ()> {
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
