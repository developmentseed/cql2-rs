use crate::{geometry::spatial_op, precedence, temporal::temporal_op, Error, Geometry, Validator};
use geo_types::{coord, Geometry as GGeom, Rect};
use json_dotpath::DotPaths;
use like::Like;
use pg_escape::quote_identifier;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::HashSet,
    fmt::Debug,
    ops::{Add, Deref},
    str::FromStr,
};
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
    "t_metBy",
    "t_overlaps",
    "t_overlappedBy",
    "t_starts",
    "t_startedBy",
    "t_during",
    "t_contains",
    "t_finishes",
    "t_finishedBy",
    "t_equals",
    "t_disjoint",
    "t_intersects",
];

/// Arithmetic Operators
pub const ARITHOPS: &[&str] = &["+", "-", "*", "/", "%", "^", "div"];

/// Array Operators
pub const ARRAYOPS: &[&str] = &["a_equals", "a_contains", "a_containedBy", "a_overlaps"];

// todo: array ops, in, casei, accenti, between, not, like
/// Operator names that belong to none of the categories above.
const OTHER_OPS: &[&str] = &["not", "like", "between", "in", "isNull", "casei", "accenti"];

/// Every operator name the JSON schema defines, in the spelling it defines.
///
/// Assembled from the category constants so each name is written once; `canonical_ops_match_the_schema`
/// pins the result against the schema itself.
fn canonical_ops() -> impl Iterator<Item = &'static str> {
    [
        BOOLOPS,
        EQOPS,
        CMPOPS,
        SPATIALOPS,
        TEMPORALOPS,
        ARITHOPS,
        ARRAYOPS,
        OTHER_OPS,
    ]
    .into_iter()
    .flat_map(|ops| ops.iter().copied())
}

/// Puts an expression into the crate's canonical form.
///
/// Both encodings run through this, so cql2-text and cql2-json describe the same expression
/// identically:
///
/// - `and` and `or` are associative, so a chain becomes one n-ary operation. A chain also has to be
///   flat to survive a text round trip, since the renderers omit the parentheses that would
///   otherwise be the only record of the nesting.
/// - A timestamp denotes an instant, so each instant has one spelling.
/// - Every operator name is resolved to its canonical spelling, which is what makes the
///   case-sensitive spellings the cql2-json schema requires come out right.
pub(crate) fn normalize(expr: Expr) -> Expr {
    match expr {
        Expr::Operation { op, args } => {
            let op = canonical_op(&op);
            let args = args.into_iter().map(|arg| Box::new(normalize(*arg)));
            if op == "and" || op == "or" {
                let mut flat: Vec<Box<Expr>> = Vec::new();
                for arg in args {
                    match *arg {
                        Expr::Operation {
                            op: nested,
                            args: inner,
                        } if nested == op => flat.extend(inner),
                        other => flat.push(Box::new(other)),
                    }
                }
                Expr::Operation { op, args: flat }
            } else {
                Expr::Operation {
                    op,
                    args: args.collect(),
                }
            }
        }
        Expr::Array(items) => {
            Expr::Array(items.into_iter().map(|i| Box::new(normalize(*i))).collect())
        }
        Expr::Timestamp { timestamp } => Expr::Timestamp {
            timestamp: Box::new(normalize_instant(*timestamp)),
        },
        Expr::Interval { interval } => Expr::Interval {
            interval: interval
                .into_iter()
                .map(|bound| Box::new(normalize_instant(*bound)))
                .collect(),
        },
        other => other,
    }
}

fn normalize_instant(expr: Expr) -> Expr {
    match expr {
        Expr::Literal(value) => Expr::Literal(crate::temporal::canonical_timestamp(&value)),
        other => normalize(other),
    }
}

/// Renders a string as a cql2-text literal: single-quoted, with an embedded quote doubled.
fn literal(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

/// Names the cql2-text grammar reads as something other than an identifier.
///
/// `Literal` is tried before `Identifier`, so a bare `true`, `false` or `null` comes back as that
/// value rather than as a name, and `not` is consumed as the prefix operator, leaving nothing for
/// the expression that follows. Every other keyword — `and`, `is`, `like`, `between`, `div` — reads
/// as an identifier where one is expected, so only these four need the quotes.
const GRAMMAR_RESERVED: &[&str] = &["true", "false", "null", "not"];

/// Renders a name as a cql2-text identifier, quoting it only where the grammar requires.
///
/// [`quote_identifier`] applies PostgreSQL's rules, which quote any name that is not lowercase, and
/// every SQL keyword besides. A cql2-text identifier is case-sensitive and admits `_`, `.` and `:`,
/// so names like `t_metBy`, `Foo` and `landsat:scene_id` are written bare. This is what both a
/// property and a function name are rendered with, so one name has one spelling wherever it appears.
fn identifier(name: &str) -> String {
    let mut chars = name.chars();
    let is_bare = chars.next().is_some_and(|c| c.is_ascii_alphabetic())
        && chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | ':'))
        && !GRAMMAR_RESERVED
            .iter()
            .any(|reserved| reserved.eq_ignore_ascii_case(name));
    if is_bare {
        name.to_string()
    } else {
        quote_identifier(name).to_string()
    }
}

/// Resolves an operator name to its canonical CQL2 spelling.
///
/// Operator names are case-insensitive, so `T_METBY` and `t_metBy` name the same operator, and the
/// grammar accepts one spelling CQL2 writes differently. Anything CQL2 does not define is a
/// user-supplied function name, returned unchanged because its case is the author's to choose.
pub(crate) fn canonical_op(name: &str) -> String {
    let aliased = ALIASES
        .iter()
        .find(|(alias, _)| alias.eq_ignore_ascii_case(name))
        .map_or(name, |(_, canonical)| canonical);
    canonical_ops()
        .find(|canonical| canonical.eq_ignore_ascii_case(aliased))
        .map_or_else(|| aliased.to_string(), str::to_string)
}

/// Alternate spellings of operators CQL2 does define, resolved case-insensitively.
///
/// Resolved here rather than in one backend, so that a single fold serves the evaluator, both
/// renderers and both encodings: `ST_Intersects(a, b)` is the same expression as
/// `s_intersects(a, b)` whichever of them reads it.
///
/// `eq`, `lt`, `ne` and friends are deliberately absent: the schema defines no such operators, so
/// they can only arrive as user-defined function names and must be left alone. `div` is likewise not
/// aliased to `/` — CQL2 defines it as integer division, a distinct operator.
const ALIASES: &[(&str, &str)] = &[
    // A spelling the cql2-text grammar itself accepts: `NotEq = { "<>" | "!=" }`.
    ("!=", "<>"),
    // How SQL and PostGIS spell the spatial predicates. One per `SPATIALOPS` entry, which
    // `every_spatial_operator_has_an_st_alias` pins.
    ("st_equals", "s_equals"),
    ("st_intersects", "s_intersects"),
    ("st_disjoint", "s_disjoint"),
    ("st_touches", "s_touches"),
    ("st_within", "s_within"),
    ("st_overlaps", "s_overlaps"),
    ("st_crosses", "s_crosses"),
    ("st_contains", "s_contains"),
    // How the earlier drafts of CQL2 spelled the two `intersects` predicates.
    ("intersects", "s_intersects"),
    ("anyinteracts", "t_intersects"),
];

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
    Null,
}
impl TryFrom<Value> for Expr {
    type Error = Error;
    /// Normalizes, so an expression built from a `Value` is identical to the same expression parsed
    /// from cql2-json text. The bindings construct expressions this way.
    fn try_from(v: Value) -> Result<Expr, Error> {
        serde_json::from_value(v)
            .map(normalize)
            .map_err(Error::from)
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
            Expr::Geometry(ref g) => {
                GGeom::try_from_wkt_str(&g.to_wkt()?).map_err(|_| Error::ExprToGeom(v.clone()))
            }
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
        "a_equals" => left == right,
        "a_contains" => left.is_superset(&right),
        "a_containedBy" => left.is_subset(&right),
        "a_overlaps" => !left.is_disjoint(&right),
        _ => return Err(Error::OperationError()),
    };
    Ok(Expr::Bool(out))
}

/// Returns `true` if a *reduced* expression is still "unknown", i.e. its value
/// cannot be determined at reduction time.
///
/// This is the case for an unresolved property reference (a property that was
/// not found in the supplied JSON, or when no JSON was supplied) or an operation
/// that could not be folded to a concrete value. Predicates over unknown
/// operands must not be constant-folded, otherwise `reduce` would invent a
/// truth value for something it does not actually know.
fn is_unknown(expr: &Expr) -> bool {
    match expr {
        Expr::Property { .. } | Expr::Operation { .. } => true,
        Expr::Interval { interval } => interval.iter().any(|e| is_unknown(e)),
        Expr::Date { date } => is_unknown(date),
        Expr::Timestamp { timestamp } => is_unknown(timestamp),
        Expr::Array(elements) => elements.iter().any(|e| is_unknown(e)),
        Expr::BBox { bbox } => bbox.iter().any(|e| is_unknown(e)),
        Expr::Float(_) | Expr::Literal(_) | Expr::Bool(_) | Expr::Geometry(_) | Expr::Null => false,
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
            Expr::Interval { ref interval } => {
                let start = interval[0].as_ref().clone().reduce(j)?;
                let end = interval[1].as_ref().clone().reduce(j)?;
                Ok(Expr::Interval {
                    interval: vec![Box::new(start), Box::new(end)],
                })
            }
            Expr::Operation { op, args } => {
                // Dispatch below matches canonical spellings, and an expression left unfolded keeps
                // the name it came in with.
                let op = canonical_op(&op);

                let args: Vec<Box<Expr>> = args
                    .into_iter()
                    .map(|expr| expr.reduce(j).map(Box::new))
                    .collect::<Result<_, _>>()?;

                if op == "isNull" {
                    if matches!(args[0].as_ref(), Expr::Null) {
                        Ok(Expr::Bool(true))
                    } else if is_unknown(args[0].as_ref()) {
                        if j.is_some() {
                            // We are reducing against a concrete record: an
                            // unresolved property means the field is absent (and
                            // therefore null) for this record, so IS NULL is true.
                            Ok(Expr::Bool(true))
                        } else {
                            // No data context: the value of the operand is unknown,
                            // so leave the predicate in place rather than folding it
                            // to a constant.
                            Ok(Expr::Operation {
                                op: "isNull".to_string(),
                                args,
                            })
                        }
                    } else {
                        Ok(Expr::Bool(false))
                    }
                } else if args.iter().any(|arg| matches!(arg.as_ref(), Expr::Null)) {
                    Ok(Expr::Bool(false))
                } else if BOOLOPS.contains(&op.as_str()) {
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
                    if args.iter().any(|a| is_unknown(a)) {
                        // One of the operands is unknown, so we can't evaluate the
                        // range check; leave the predicate in place.
                        Ok(Expr::Operation { op, args })
                    } else {
                        Ok(Expr::Bool(args[0] >= args[1] && args[0] <= args[2]))
                    }
                } else if args.len() != 2 {
                    Ok(Expr::Operation { op, args })
                } else {
                    // Two-arg operations
                    let mut left = args[0].deref().clone();
                    let mut right = args[1].deref().clone();

                    // If either operand is unknown (an unresolved property or an
                    // expression that did not fold to a concrete value) we cannot
                    // evaluate the operation, so leave it in place rather than
                    // constant-folding it to an incorrect value.
                    if is_unknown(&left) || is_unknown(&right) {
                        return Ok(Expr::Operation { op, args });
                    }

                    // If left is Date/Timestamp and right is Literal, convert right
                    match (&left, &right) {
                        (Expr::Date { .. }, Expr::Literal(ref v)) => {
                            right = Expr::Date {
                                date: Box::new(Expr::Literal(v.clone())),
                            };
                        }
                        (Expr::Timestamp { .. }, Expr::Literal(ref v)) => {
                            right = Expr::Timestamp {
                                timestamp: Box::new(Expr::Literal(v.clone())),
                            };
                        }
                        (Expr::Literal(ref v), Expr::Date { .. }) => {
                            left = Expr::Date {
                                date: Box::new(Expr::Literal(v.clone())),
                            };
                        }
                        (Expr::Literal(ref v), Expr::Timestamp { .. }) => {
                            left = Expr::Timestamp {
                                timestamp: Box::new(Expr::Literal(v.clone())),
                            };
                        }
                        _ => {}
                    }

                    if TEMPORALOPS.contains(&op.as_str()) {
                        Ok(temporal_op(left, right, &op)
                            .unwrap_or_else(|_| Expr::Operation { op, args }))
                    // Date or Timestamp comparison: convert to jiff Timestamp for correct ordering
                    } else if (matches!(left, Expr::Date { .. } | Expr::Timestamp { .. })
                        && matches!(right, Expr::Date { .. } | Expr::Timestamp { .. })
                        && (EQOPS.contains(&op.as_str()) || CMPOPS.contains(&op.as_str())))
                    {
                        // convert both operands to DateRange and compare using PartialOrd/PartialEq
                        let l_dr = crate::temporal::DateRange::try_from(left.clone())?;
                        let r_dr = crate::temporal::DateRange::try_from(right.clone())?;
                        let cmp = match op.as_str() {
                            "=" => l_dr == r_dr,
                            "<=" => l_dr <= r_dr,
                            "<" => l_dr < r_dr,
                            ">=" => l_dr >= r_dr,
                            ">" => l_dr > r_dr,
                            "<>" => l_dr != r_dr,
                            _ => unreachable!(),
                        };
                        Ok(Expr::Bool(cmp))
                    } else if std::mem::discriminant(&left) == std::mem::discriminant(&right) {
                        if SPATIALOPS.contains(&op.as_str()) {
                            Ok(spatial_op(left, right, &op)
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

    /// Returns True if the expression evaluates to true and false if either the expression evaluates to false or does not fully reduce to a boolean.
    pub fn is_true(self) -> bool {
        matches!(self, Expr::Bool(true))
    }

    /// Filters an iterable of JSON values based on this expression.
    ///
    /// # Examples
    ///
    /// ```
    /// use cql2::Expr;
    /// use serde_json::json;
    ///
    /// let expr: Expr = "eo:cloud_cover < 20".parse().unwrap();
    /// let items = vec![
    ///     json!({"properties": {"eo:cloud_cover": 10}}),
    ///     json!({"properties": {"eo:cloud_cover": 25}}),
    ///     json!({"properties": {"eo:cloud_cover": 15}})
    /// ];
    /// let filtered = expr.filter(&items).unwrap();
    /// assert_eq!(filtered.len(), 2);
    /// assert_eq!(filtered[0]["properties"]["eo:cloud_cover"], 10);
    /// assert_eq!(filtered[1]["properties"]["eo:cloud_cover"], 15);
    /// ```
    pub fn filter<'a, I>(&self, items: I) -> Result<Vec<&'a Value>, Error>
    where
        I: IntoIterator<Item = &'a Value>,
    {
        let mut filtered = Vec::new();
        for item in items {
            let e = self.clone().reduce(Some(item))?;
            if e.is_true() {
                filtered.push(item)
            }
        }
        Ok(filtered)
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
            // An infinity or a NaN has no cql2-text literal, and the bare words `inf` and `NaN` the
            // default rendering writes would parse back as property names rather than as numbers.
            Expr::Float(v) if !v.is_finite() => Err(Error::NonFiniteNumber(*v)),
            Expr::Float(v) => Ok(v.to_string()),
            Expr::Literal(v) => Ok(literal(v)),
            Expr::Property { property } => Ok(identifier(property)),
            Expr::Null => Ok("NULL".to_string()),
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
                // Dispatch on the canonical spelling so a tree built without `normalize` renders
                // the same as a parsed one.
                let op = canonical_op(op);
                // Parenthesize only the operands that would otherwise re-associate when the text is
                // parsed back, so the rendering round-trips to an identical expression.
                let requirement = precedence::operands(&op);
                let a: Vec<String> = args
                    .iter()
                    .enumerate()
                    .map(|(index, arg)| {
                        let text = arg.to_text()?;
                        Ok(if requirement.needs_parens(index, arg) {
                            format!("({})", text)
                        } else {
                            text
                        })
                    })
                    .collect::<Result<_, Error>>()?;
                match op.as_str() {
                    "and" => Ok(a.join(" AND ")),
                    "or" => Ok(a.join(" OR ")),
                    "like" => {
                        check_len!("like", a, 2, format!("{} LIKE {}", a[0], a[1]))
                    }
                    "in" => {
                        check_len!("in", a, 2, format!("{} IN {}", a[0], a[1]))
                    }
                    "between" => {
                        check_len!(
                            "between",
                            a,
                            3,
                            format!("{} BETWEEN {} AND {}", a[0], a[1], a[2])
                        )
                    }
                    "not" => {
                        check_len!("not", a, 1, format!("NOT {}", a[0]))
                    }
                    "isNull" => {
                        check_len!("is null", a, 1, format!("{} IS NULL", a[0]))
                    }
                    "+" | "-" | "*" | "/" | "%" => {
                        let paddedop = format!(" {} ", op);
                        Ok(a.join(&paddedop).to_string())
                    }
                    "^" | "=" | "<=" | "<" | "<>" | ">" | ">=" => {
                        check_len!(op, a, 2, format!("{} {} {}", a[0], op, a[1]))
                    }
                    _ => Ok(format!("{}({})", identifier(&op), a.join(", "))),
                }
            }
            Expr::BBox { bbox } => {
                let array_els: Vec<String> =
                    bbox.iter().map(|a| a.to_text()).collect::<Result<_, _>>()?;
                Ok(format!("BBOX({})", array_els.join(", ")))
            }
        }
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
    use super::{canonical_op, canonical_ops, ALIASES, SPATIALOPS};
    use crate::Expr;
    use serde_json::Value;
    use std::collections::HashSet;

    /// Every spatial predicate is reachable by its SQL/PostGIS spelling, not just the ones that
    /// happened to be written down.
    #[test]
    fn every_spatial_operator_has_an_st_alias() {
        for op in SPATIALOPS {
            let st = format!("st_{}", op.trim_start_matches("s_"));
            assert_eq!(canonical_op(&st), *op, "'{st}' does not resolve to '{op}'");
        }
    }

    /// An alias has to name an operator that exists, or it silently invents one.
    #[test]
    fn aliases_resolve_to_canonical_operators() {
        let known: HashSet<&str> = canonical_ops().collect();
        for (alias, canonical) in ALIASES {
            assert!(
                known.contains(canonical),
                "'{alias}' resolves to '{canonical}', which is not an operator"
            );
        }
    }

    /// Aliases fold in both encodings, so the evaluator and both renderers see one operator.
    #[test]
    fn aliases_fold_in_both_encodings() {
        for (source, expected) in [
            ("ST_Intersects(geom, POINT(0 0))", "s_intersects"),
            ("st_intersects(geom, POINT(0 0))", "s_intersects"),
            ("INTERSECTS(geom, POINT(0 0))", "s_intersects"),
            ("AnyInteracts(a, b)", "t_intersects"),
            ("ST_CONTAINS(geom, POINT(0 0))", "s_contains"),
        ] {
            for text in [
                source.to_string(),
                // The same expression in cql2-json, built from the text spelling.
                {
                    let name = source.split('(').next().expect("has a name");
                    format!(r#"{{"op":"{name}","args":[{{"property":"a"}},{{"property":"b"}}]}}"#)
                },
            ] {
                let Ok(Expr::Operation { op, .. }) = text.parse::<Expr>() else {
                    panic!("{text} should parse to an operation");
                };
                assert_eq!(op, expected, "{text} did not fold to {expected}");
            }
        }
    }

    /// An infinity or a NaN has no cql2-text spelling, so rendering one is an error.
    ///
    /// Rendered as the bare word `f64::to_string` writes, `inf` would parse back as a *property*
    /// named `inf` — a different expression that reads as valid CQL2, which is worse than a failure.
    #[test]
    fn non_finite_numbers_have_no_text() {
        for value in [f64::INFINITY, f64::NEG_INFINITY, f64::NAN] {
            assert!(
                matches!(
                    Expr::Float(value).to_text(),
                    Err(crate::Error::NonFiniteNumber(_))
                ),
                "{value} rendered as cql2-text"
            );
        }
        // Reachable from an expression that parsed: division by zero reduces to an infinity.
        let divided: Expr = "1 / 0".parse().unwrap();
        assert!(matches!(
            divided.reduce(None).unwrap().to_text(),
            Err(crate::Error::NonFiniteNumber(_))
        ));
        // An operand buried in a larger expression is reported the same way.
        let expr = Expr::Operation {
            op: ">".to_string(),
            args: vec![
                Box::new(Expr::Property {
                    property: "a".to_string(),
                }),
                Box::new(Expr::Float(f64::INFINITY)),
            ],
        };
        assert!(matches!(
            expr.to_text(),
            Err(crate::Error::NonFiniteNumber(_))
        ));
    }

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

    #[test]
    fn keep_one_element_lists() {
        // A single-element list stays a list rather than collapsing to a bare value.
        let expr: Expr = "ogc_fid IN ('1')".parse().unwrap();
        assert_eq!(expr.to_text().unwrap(), "ogc_fid IN ('1')");
    }

    /// The operator constants decide the spelling every name is resolved to, and the JSON schema
    /// enumerates those names case-sensitively. An operator missing here is emitted with whatever
    /// case it was written in, which the schema then reads as a function call rather than an
    /// operator, so the two lists have to agree.
    #[test]
    fn canonical_ops_match_the_schema() {
        let schema: Value =
            serde_json::from_str(include_str!("cql2.json")).expect("schema is valid JSON");

        let mut from_schema = HashSet::new();
        collect_operator_enums(&schema, &mut from_schema);
        assert!(
            !from_schema.is_empty(),
            "found no operator enums in the schema"
        );

        let known: HashSet<String> = canonical_ops().map(str::to_string).collect();
        let missing: Vec<&String> = from_schema.difference(&known).collect();
        assert!(
            missing.is_empty(),
            "these schema operators are absent from the operator constants: {missing:?}"
        );
    }

    /// Collects every `enum` in the schema that names operators, identified by containing an
    /// operator this crate is certain of.
    fn collect_operator_enums(node: &Value, out: &mut HashSet<String>) {
        match node {
            Value::Object(fields) => {
                if let Some(Value::Array(values)) = fields.get("enum") {
                    let names: Vec<String> = values
                        .iter()
                        .filter_map(|v| v.as_str().map(str::to_string))
                        .collect();
                    if names.iter().any(|n| n == "t_metBy" || n == "a_containedBy") {
                        out.extend(names);
                    }
                }
                for value in fields.values() {
                    collect_operator_enums(value, out);
                }
            }
            Value::Array(items) => items
                .iter()
                .for_each(|item| collect_operator_enums(item, out)),
            _ => {}
        }
    }
}
