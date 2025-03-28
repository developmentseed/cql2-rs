use crate::expr::{ARITHOPS, ARRAYOPS, BOOLOPS, CMPOPS, EQOPS, SPATIALOPS, TEMPORALOPS};
use crate::Error;
use crate::Expr;
use crate::Geometry;
use pg_escape::{quote_identifier, quote_literal};

/// Traits for generating SQL for DuckDB with Spatial Extension
pub trait ToDuckSQL {
    /// Convert Expression to SQL for DuckDB with Spatial Extension
    fn to_ducksql(&self) -> Result<String, Error>;
}

fn lit_or_prop_to_ts(arg: &Expr) -> Result<String, Error> {
    match arg {
        Expr::Property { property } => Ok(quote_identifier(property).to_string()),
        Expr::Literal(v) => Ok(format!("TIMESTAMP {}", quote_literal(v))),
        _ => Err(Error::OperationError()),
    }
}

impl ToDuckSQL for Expr {
    /// Converts this expression to DuckDB SQL.
    /// WARNING: This is an experimental feature with limited tests subject to change!
    ///
    /// # Examples
    ///
    /// ```
    /// use cql2::Expr;
    /// use cql2::ToDuckSQL;
    ///
    /// let expr = Expr::Bool(true);
    /// assert_eq!(expr.to_ducksql().unwrap(), "true");
    /// ```
    /// ```
    /// use cql2::Expr;
    /// use cql2::ToDuckSQL;
    ///
    /// let expr: Expr = "s_intersects(geom, POINT(0 0)) and foo >= 1 and bar='baz' and TIMESTAMP('2020-01-01 00:00:00Z') >= BoRk".parse().unwrap();
    /// assert_eq!(expr.to_ducksql().unwrap(), "ST_intersects(geom,ST_GeomFromText('POINT(0 0)')) and foo >= 1 and bar = 'baz' and TIMESTAMPTZ '2020-01-01 00:00:00Z' >= \"BoRk\"");
    /// ```
    /// ```
    /// use cql2::Expr;
    /// use cql2::ToDuckSQL;
    ///
    /// let expr: Expr = "t_overlaps(interval(a,'2020-01-01T00:00:00Z'),interval('2020-01-01T00:00:00Z','2020-02-01T00:00:00Z'))".parse().unwrap();
    /// assert_eq!(expr.to_ducksql().unwrap(), "a < TIMESTAMP '2020-02-01T00:00:00Z' AND TIMESTAMP '2020-01-01T00:00:00Z' < TIMESTAMP '2020-01-01T00:00:00Z' AND TIMESTAMP '2020-01-01T00:00:00Z' < TIMESTAMP '2020-02-01T00:00:00Z'");
    /// ```
    /// ```
    /// use cql2::Expr;
    /// use cql2::ToDuckSQL;
    ///
    /// let expr: Expr = "t_overlaps(interval(a,b),interval('2020-01-01T00:00:00Z','2020-02-01T00:00:00Z'))".parse().unwrap();
    /// assert_eq!(expr.to_ducksql().unwrap(), "a < TIMESTAMP '2020-02-01T00:00:00Z' AND TIMESTAMP '2020-01-01T00:00:00Z' < b AND b < TIMESTAMP '2020-02-01T00:00:00Z'");
    /// ```
    fn to_ducksql(&self) -> Result<String, Error> {
        Ok(match self {
            Expr::Bool(v) => {
                format!("{v}")
            }
            Expr::Float(v) => {
                format!("{v}")
            }
            Expr::Literal(v) => quote_literal(v).to_string(),
            Expr::Date { date } => {
                let s = date.to_ducksql()?;
                format!("DATE {s}")
            }
            Expr::Timestamp { timestamp } => {
                let s = timestamp.to_ducksql()?;
                format!("TIMESTAMPTZ {s}")
            }
            Expr::Interval { interval } => {
                let start = lit_or_prop_to_ts(interval[0].as_ref())?;
                let end = lit_or_prop_to_ts(interval[1].as_ref())?;
                format!("array_value({start}, {end})")
            }
            Expr::Geometry(v) => match v {
                Geometry::GeoJSON(v) => {
                    let s = v.to_string();
                    format!("ST_GeomFromGeoJSON({})", quote_literal(&s))
                }
                Geometry::Wkt(v) => {
                    format!("ST_GeomFromText({})", quote_literal(v))
                }
            },

            Expr::BBox { bbox } => {
                let array_els: Vec<String> = bbox
                    .iter()
                    .map(|a| a.to_ducksql())
                    .collect::<Result<_, _>>()?;
                format!("[{}]", array_els.join(", "))
            }
            Expr::Array(v) => {
                let array_els: Vec<String> =
                    v.iter().map(|a| a.to_ducksql()).collect::<Result<_, _>>()?;
                format!("array_value({})", array_els.join(", "))
            }
            Expr::Property { property } => format!("{}", quote_identifier(property)),
            Expr::Operation { op, args } => {
                let a: Vec<String> = args
                    .iter()
                    .map(|x| x.to_ducksql())
                    .collect::<Result<_, _>>()?;
                let op = op.as_str();
                match op {
                    "not" => format!("NOT {}", a[0]),
                    "between" => format!("{} BETWEEN {} AND {}", a[0], a[1], a[2]),
                    "in" => format!("IN ({})", a.join(",")),
                    "like" => format!("{} LIKE {}", a[0], a[1]),
                    "accenti" => format!("strip_accents({})", a[0]),
                    "casei" => format!("lower({})", a[0]),
                    _ => {
                        if BOOLOPS.contains(&op) {
                            let padded = format!(" {} ", op);
                            a.join(&padded).to_string()
                        } else if SPATIALOPS.contains(&op) {
                            let sop = op.strip_prefix("s_").unwrap();
                            format!("ST_{}({},{})", sop, a[0], a[1])
                        } else if ARRAYOPS.contains(&op) {
                            match op {
                                "a_equals" => format!("{} = {}", a[0], a[1]),
                                "a_contains" => format!("list_has_all({},{})", a[0], a[1]),
                                "a_containedby" => format!("list_has_all({},{})", a[1], a[2]),
                                "a_overlaps" => format!("list_has_any({},{})", a[0], a[1]),
                                _ => unreachable!(),
                            }
                        } else if TEMPORALOPS.contains(&op) {
                            let left_expr = *args[0].clone();
                            let right_expr = *args[1].clone();

                            let left_start_init: String;
                            let left_end_init: String;
                            let right_start_init: String;
                            let right_end_init: String;

                            if let Expr::Interval { interval } = left_expr {
                                left_start_init = lit_or_prop_to_ts(&interval[0])?;
                                left_end_init = lit_or_prop_to_ts(&interval[1])?;
                            } else {
                                unreachable!()
                            }

                            if let Expr::Interval { interval } = right_expr {
                                right_start_init = lit_or_prop_to_ts(&interval[0])?;
                                right_end_init = lit_or_prop_to_ts(&interval[1])?;
                            } else {
                                unreachable!()
                            }

                            let invop = match op {
                                "t_after" => "t_before",
                                "t_metby" => "t_meets",
                                "t_overlappedby" => "t_overlaps",
                                "t_startedby" => "t_starts",
                                "t_contains" => "t_during",
                                "t_finishedby" => "t_finishes",
                                _ => op,
                            };

                            let left_start: &str;
                            let left_end: &str;
                            let right_start: &str;
                            let right_end: &str;

                            if invop == op {
                                left_start = &left_start_init;
                                left_end = &left_end_init;
                                right_start = &right_start_init;
                                right_end = &right_end_init;
                            } else {
                                right_start = &left_start_init;
                                right_end = &left_end_init;
                                left_start = &right_start_init;
                                left_end = &right_end_init;
                            }

                            match invop {
                                "t_before" => format!("{left_end} < {right_start}"),
                                "t_meets" => format!("{left_end} = {right_start}"),
                                "t_overlaps" => {
                                    format!("{left_start} < {right_end} AND {right_start} < {left_end} AND {left_end} < {right_end}")
                                }
                                "t_starts" => format!("{left_start} = {right_start} AND {left_end} < {right_end}"),
                                "t_during" => format!("{left_start} > {right_start} AND {left_end} < {right_end}"),
                                "t_finishes" => format!("{left_start} > {right_start} AND {left_end} = {right_end}"),
                                "t_equals" => format!("{left_start} = {right_start} AND {left_end} = {right_end}"),
                                "t_disjoint" => format!("NOT ({left_start} <= {right_end} AND {left_end} >= {right_start})"),
                                "t_intersects" | "anyinteracts" => format!("{left_start} <= {right_end} AND {left_end} >= {right_start}"),
                                _ => unreachable!()
                            }
                        } else if CMPOPS.contains(&op)
                            || EQOPS.contains(&op)
                            || ARITHOPS.contains(&op)
                        {
                            format!("{} {} {}", a[0], op, a[1])
                        } else {
                            unreachable!()
                        }
                    }
                }
            }
        })
    }
}
