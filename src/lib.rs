//! Parse and transform [Common Query Language 2 (CQL2)](https://docs.ogc.org/DRAFTS/21-065.html).

#![deny(unused_crate_dependencies)]

use boon::{Compiler, SchemaIndex, Schemas, ValidationError};
use geozero::{
    geojson::{GeoJsonString, GeoJsonWriter},
    wkt::Wkt,
    CoordDimensions, GeozeroGeometry, ToJson, ToWkt,
};
use pest::{
    iterators::{Pair, Pairs},
    pratt_parser::PrattParser,
    Parser,
};
use serde_derive::{Deserialize, Serialize};
use serde_json::Value;
use std::{fs, io::Read, path::Path};
use thiserror::Error;

/// Crate-specific error enum.
#[derive(Debug, Error)]
pub enum Error {
    /// [boon::CompileError]
    #[error(transparent)]
    BoonCompile(#[from] boon::CompileError),

    /// Invalid CQL2 text
    #[error("invalid cql2-text: {0}")]
    InvalidCql2Text(String),

    /// [std::io::Error]
    #[error(transparent)]
    Io(#[from] std::io::Error),

    /// [std::num::ParseIntError]
    #[error(transparent)]
    ParseInt(#[from] std::num::ParseIntError),

    /// [pest::error::Error]
    #[error(transparent)]
    Pest(#[from] Box<pest::error::Error<Rule>>),

    /// [serde_json::Error]
    #[error(transparent)]
    SerdeJson(#[from] serde_json::Error),

    /// A validation error.
    ///
    /// This holds a [serde_json::Value] that is the output from a
    /// [boon::ValidationError]. We can't hold the validation error itself
    /// becuase it contains references to both the validated object and the
    /// validator's data.
    #[error("validation error")]
    Validation(Value),
}

/// A re-usable json-schema validator for CQL2.
pub struct Validator {
    schemas: Schemas,
    index: SchemaIndex,
}

impl Validator {
    /// Creates a new validator.
    ///
    /// # Examples
    ///
    /// ```
    /// use cql2::Validator;
    ///
    /// let validator = Validator::new().unwrap();
    /// ```
    pub fn new() -> Result<Validator, Error> {
        let mut schemas = Schemas::new();
        let mut compiler = Compiler::new();
        let schema_json = serde_json::from_str(include_str!("cql2.json"))?;
        compiler.add_resource("/tmp/cql2.json", schema_json)?;
        let index = compiler.compile("/tmp/cql2.json", &mut schemas)?;
        Ok(Validator { schemas, index })
    }

    /// Validates a [serde_json::Value].
    ///
    /// # Examples
    ///
    /// ```
    /// use cql2::Validator;
    /// use serde_json::json;
    ///
    /// let validator = Validator::new().unwrap();
    ///
    /// let valid = json!({
    ///     "op": "=",
    ///     "args": [
    ///         { "property": "landsat:scene_id" },
    ///         "LC82030282019133LGN00"
    ///     ]
    /// });
    /// validator.validate(&valid).unwrap();
    ///
    /// let invalid = json!({
    ///     "op": "not an operator!",
    /// });
    /// validator.validate(&invalid).unwrap_err();
    /// ```
    pub fn validate<'a, 'b>(&'a self, value: &'b Value) -> Result<(), ValidationError<'a, 'b>> {
        self.schemas.validate(value, self.index)
    }
}

/// [pest] parser for CQL2.
#[derive(pest_derive::Parser)]
#[grammar = "cql2.pest"]
pub struct CQL2Parser;

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

/// A SQL query, broken into the query and parameters.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SqlQuery {
    query: String,
    params: Vec<String>,
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
    pub fn to_text(&self) -> Result<String, geozero::error::GeozeroError> {
        Ok(match self {
            Expr::Bool(v) => v.to_string(),
            Expr::Float(v) => v.to_string(),
            Expr::Literal(v) => format!("'{}'", v.as_str()),
            Expr::Property { property } => format!("\"{property}\""),
            Expr::Interval { interval } => format!(
                "INTERVAL({},{})",
                interval[0].to_text()?,
                interval[1].to_text()?
            ),
            Expr::Date { date } => format!("DATE({})", date.to_text()?),
            Expr::Timestamp { timestamp } => format!("TIMESTAMP({})", timestamp.to_text()?),
            Expr::Geometry(v) => {
                let gj = GeoJsonString(v.to_string());
                gj.to_wkt()?
            }
            Expr::Array(v) => {
                let array_els: Vec<String> =
                    v.iter().map(|a| a.to_text()).collect::<Result<_, _>>()?;
                format!("({})", array_els.join(", "))
            }
            Expr::Operation { op, args } => {
                let a: Vec<String> = args.iter().map(|x| x.to_text()).collect::<Result<_, _>>()?;
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
                let array_els: Vec<String> =
                    bbox.iter().map(|a| a.to_text()).collect::<Result<_, _>>()?;
                format!("BBOX({})", array_els.join(", "))
            }
        })
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

lazy_static::lazy_static! {
    static ref PRATT_PARSER: PrattParser<Rule> = {
        use pest::pratt_parser::{Assoc::*, Op};
        use Rule::*;
        PrattParser::new()
            .op(Op::infix(Or, Left))
            .op(Op::infix(Between, Left))
            .op(Op::infix(And, Left))
            .op(Op::prefix(UnaryNot))
            .op(Op::infix(Eq, Right))
            .op(
                Op::infix(NotEq, Right) |
                Op::infix(Gt, Right) |
                Op::infix(GtEq, Right) |
                Op::infix(Lt, Right) |
                Op::infix(LtEq, Right)
            )
            .op(Op::infix(Like, Right))
            .op(Op::infix(In, Left))
            .op(Op::postfix(IsNullPostfix))
            .op(Op::infix(Is, Right))
            .op(
                Op::infix(Add, Left) |
                Op::infix(Subtract, Left)
            )
            .op(
                Op::infix(Multiply, Left) |
                Op::infix(Divide, Left) |
                Op::infix(Modulo, Left)
            )
            .op(Op::infix(Power, Left))
            .op(Op::prefix(Negative))
        };
}

fn normalize_op(op: &str) -> String {
    let oper = op.to_lowercase();
    let operator: &str = match oper.as_str() {
        "eq" => "=",
        _ => &oper,
    };
    operator.to_string()
}

fn strip_quotes(quoted_string: &str) -> String {
    let len = quoted_string.len();
    let bytes = quoted_string.as_bytes();
    if (bytes[0] == b'"' && bytes[len - 1] == b'"')
        || (bytes[0] == b'\'' && bytes[len - 1] == b'\'')
    {
        quoted_string[1..len - 1].to_string()
    } else {
        quoted_string.to_string()
    }
}

fn opstr(op: Pair<Rule>) -> String {
    return normalize_op(op.as_str());
}

fn parse_expr(expression_pairs: Pairs<'_, Rule>) -> Expr {
    PRATT_PARSER
        .map_primary(|primary| match primary.as_rule() {
            Rule::Expr | Rule::ExpressionInParentheses => parse_expr(primary.into_inner()),
            Rule::Unsigned => Expr::Float(
                primary
                    .as_str()
                    .parse::<f64>()
                    .expect("Could not cast value to float"),
            ),
            Rule::DECIMAL => Expr::Float(
                primary
                    .as_str()
                    .parse::<f64>()
                    .expect("Could not cast value to float"),
            ),
            Rule::SingleQuotedString => Expr::Literal(strip_quotes(primary.as_str())),
            Rule::True | Rule::False => {
                let bool_value = primary.as_str().to_lowercase().parse::<bool>().unwrap();
                Expr::Bool(bool_value)
            }
            Rule::Identifier => Expr::Property {
                property: strip_quotes(primary.as_str()),
            },
            Rule::GEOMETRY => {
                let geom_wkt = Wkt(primary.as_str());
                let mut out: Vec<u8> = Vec::new();
                let mut p = GeoJsonWriter::with_dims(&mut out, CoordDimensions::xyz());
                let _ = geom_wkt.process_geom(&mut p);
                Expr::Geometry(serde_json::from_str(&geom_wkt.to_json().unwrap()).unwrap())
            }
            Rule::Function => {
                let mut pairs = primary.into_inner();
                let op = strip_quotes(pairs.next().unwrap().as_str()).to_lowercase();
                let mut args = Vec::new();
                for pair in pairs {
                    args.push(Box::new(parse_expr(pair.into_inner())))
                }
                match op.as_str() {
                    "interval" => Expr::Interval { interval: args },
                    "date" => Expr::Date {
                        date: args.into_iter().next().unwrap(),
                    },
                    "timestamp" => Expr::Timestamp {
                        timestamp: args.into_iter().next().unwrap(),
                    },
                    _ => Expr::Operation { op, args },
                }
            }
            Rule::Array => {
                let pairs = primary.into_inner();
                let mut array_elements = Vec::new();
                for pair in pairs {
                    array_elements.push(Box::new(parse_expr(pair.into_inner())))
                }
                Expr::Array(array_elements)
            }

            rule => unreachable!("Expr::parse expected atomic rule, found {:?}", rule),
        })
        .map_infix(|lhs, op, rhs| {
            let mut opstring = opstr(op);

            let mut notflag: bool = false;
            if opstring.starts_with("not") {
                opstring = opstring.replace("not ", "");
                notflag = true;
            }

            let origargs = vec![Box::new(lhs.clone()), Box::new(rhs.clone())];
            let mut retexpr: Expr;
            let mut lhsclone = lhs.clone();
            let rhsclone = rhs.clone();

            let mut lhsargs: Vec<Box<Expr>> = Vec::new();
            let mut rhsargs: Vec<Box<Expr>> = Vec::new();
            let mut betweenargs: Vec<Box<Expr>> = Vec::new();

            if opstring == "between" {
                match &lhsclone {
                    Expr::Operation { op, args } if op == "and" => {
                        lhsargs = args.to_vec();
                        lhsclone = *lhsargs.pop().unwrap();
                    }
                    _ => (),
                }

                match &lhsclone {
                    Expr::Operation { op, args } if op == "not" => {
                        lhsargs = args.to_vec();
                        lhsclone = *lhsargs.pop().unwrap();
                        notflag = true;
                    }
                    _ => (),
                }
                let betweenleft = lhsclone.to_owned();
                betweenargs.push(Box::new(betweenleft));

                match &rhs {
                    Expr::Operation { op, args } if op == "and" => {
                        for a in args {
                            betweenargs.push(a.clone());
                        }
                        rhsargs = betweenargs.split_off(3);
                    }
                    _ => (),
                }

                retexpr = Expr::Operation {
                    op: "between".to_string(),
                    args: betweenargs,
                };

                if notflag {
                    retexpr = Expr::Operation {
                        op: "not".to_string(),
                        args: vec![Box::new(retexpr)],
                    };
                };

                if lhsargs.is_empty() || rhsargs.is_empty() {
                    return retexpr;
                }

                let mut andargs: Vec<Box<Expr>> = Vec::new();

                if !lhsargs.is_empty() {
                    for a in lhsargs.into_iter() {
                        andargs.push(a);
                    }
                }
                andargs.push(Box::new(retexpr));

                if !rhsargs.is_empty() {
                    for a in rhsargs.into_iter() {
                        andargs.push(a);
                    }
                }

                return Expr::Operation {
                    op: "and".to_string(),
                    args: andargs,
                };
            } else {
                let mut outargs: Vec<Box<Expr>> = Vec::new();

                match lhsclone {
                    Expr::Operation { ref op, ref args } if op == "and" => {
                        for arg in args.iter() {
                            outargs.push(arg.clone());
                        }
                        outargs.push(Box::new(rhsclone));
                        return Expr::Operation {
                            op: "and".to_string(),
                            args: outargs,
                        };
                    }
                    _ => (),
                }
                retexpr = Expr::Operation {
                    op: opstring,
                    args: origargs,
                };
            }

            if notflag {
                return Expr::Operation {
                    op: "not".to_string(),
                    args: vec![Box::new(retexpr)],
                };
            }
            retexpr
        })
        .map_prefix(|op, child| match op.as_rule() {
            Rule::UnaryNot => Expr::Operation {
                op: "not".to_string(),
                args: vec![Box::new(child)],
            },
            Rule::Negative => Expr::Operation {
                op: "*".to_string(),
                args: vec![Box::new(Expr::Float(-1.0)), Box::new(child)],
            },
            rule => unreachable!("Expr::parse expected prefix operator, found {:?}", rule),
        })
        .map_postfix(|child, op| match op.as_rule() {
            Rule::IsNullPostfix => Expr::Operation {
                op: "isNull".to_string(),
                args: vec![Box::new(child)],
            },
            rule => unreachable!("Expr::parse expected postfix operator, found {:?}", rule),
        })
        .parse(expression_pairs)
}

/// Parses a string into a CQL2 expression.
///
/// The string can be cql2-text or cql2-json — the type will be auto-detected.
/// Use [parse_text] and [parse_json] if you already know the CQL2 type of the
/// string.
///
/// # Examples
///
/// ```
/// let s = "landsat:scene_id = 'LC82030282019133LGN00'";
/// let expr = cql2::parse(s);
/// ```
pub fn parse(cql2: &str) -> Result<Expr, Error> {
    // TODO impl FromStr
    if cql2.starts_with('{') {
        parse_json(cql2).map_err(Error::from)
    } else {
        parse_text(cql2)
    }
}

/// Parses a cql2-json string into a CQL2 expression.
///
/// # Examples
///
/// ```
/// let s = include_str!("../tests/fixtures/json/example01.json");
/// let expr = cql2::parse_json(s);
/// ```
pub fn parse_json(cql2: &str) -> Result<Expr, serde_json::Error> {
    serde_json::from_str(cql2)
}

/// Parses a cql2-text string into a CQL2 expression.
///
/// # Examples
///
/// ```
/// let s = "landsat:scene_id = 'LC82030282019133LGN00'";
/// let expr = cql2::parse_text(s);
/// ```
pub fn parse_text(cql2: &str) -> Result<Expr, Error> {
    let mut pairs = CQL2Parser::parse(Rule::Expr, cql2).map_err(Box::new)?;
    if let Some(pair) = pairs.next() {
        if pairs.next().is_some() {
            Err(Error::InvalidCql2Text(cql2.to_string()))
        } else {
            Ok(parse_expr(pair.into_inner()))
        }
    } else {
        Err(Error::InvalidCql2Text(cql2.to_string()))
    }
}

/// Reads a file and returns its contents as a CQL2 expression;
///
/// # Examples
///
/// ```no_run
/// let expr = cql2::parse_file("tests/fixtures/json/example01.json");
/// ```
pub fn parse_file(path: impl AsRef<Path>) -> Result<Expr, Error> {
    let cql2 = fs::read_to_string(path)?;
    parse(&cql2)
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

fn parse_stderr(cql2: &str) -> Result<Expr, Error> {
    let debug_level: u8 = std::env::var("CQL2_DEBUG_LEVEL")
        .ok()
        .map(|s| s.parse())
        .transpose()?
        .unwrap_or(1);
    let validator = Validator::new().unwrap();

    let parsed: Expr = parse(cql2)?;
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
