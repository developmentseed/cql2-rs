use crate::Expr;
use crate::expr::{BOOLOPS, EQOPS, CMPOPS, SPATIALOPS, TEMPORALOPS, ARITHOPS, ARRAYOPS};
use crate::Error;
use crate::Geometry;
use pg_escape::{quote_identifier, quote_literal};



pub trait ToDuckSQL {
    fn to_ducksql(&self) -> Result<String, Error>;
}


impl ToDuckSQL for Expr {
    /// Converts this expression to DuckDB SQL.
    ///
    /// # Examples
    ///
    /// ```
    /// use cql2::Expr;
    /// use cql2::ToDuckSQL;
    ///
    /// let expr = Expr::Bool(true);
    /// assert_eq!(expr.to_ducksql().unwrap(), "true");
    ///
    /// let expr: Expr = "s_intersects(geom, POINT(0 0)) and foo >= 1 and bar='baz' and TIMESTAMP('2020-01-01 00:00:00Z') >= BoRk".parse().unwrap();
    /// assert_eq!(expr.to_ducksql().unwrap(), "ST_intersects(geom,ST_GeomFromText('POINT(0 0)')) and foo >= 1 and bar = 'baz' and TIMESTAMPTZ '2020-01-01 00:00:00Z' >= \"BoRk\"");
    /// ```
    fn to_ducksql(&self) -> Result<String, Error>{
        Ok(match self {
            Expr::Bool(v) => {
                format!("{v}")
            },
            Expr::Float(v) =>  {
                format!("{v}")
            },
            Expr::Literal(v) => {
                format!("{}", quote_literal(v))
            },
            Expr::Date { date } => {
                let s = date.to_ducksql()?;
                format!("DATE {s}")
            },
            Expr::Timestamp { timestamp } => {
                let s = timestamp.to_ducksql()?;
                format!("TIMESTAMPTZ {s}")
            },
            Expr::Interval { interval } => todo!(),
            Expr::Geometry(v) => {
                match v {
                    Geometry::GeoJSON(v) => {
                        let s = v.to_string();
                        format!("ST_GeomFromGeoJSON({})", quote_literal(&s))
                    },
                    Geometry::Wkt(v)  => {
                        format!("ST_GeomFromText({})", quote_literal(v))
                    }
                }
            },

            Expr::BBox { bbox } => {
                let array_els: Vec<String> = bbox
                    .iter()
                    .map(|a| a.to_ducksql())
                    .collect::<Result<_, _>>()?;
                format!("[{}]", array_els.join(", "))
            },
            Expr::Array(v) => {
                let array_els: Vec<String> = v
                    .iter()
                    .map(|a| a.to_ducksql())
                    .collect::<Result<_, _>>()?;
                format!("[{}]", array_els.join(", "))
            },
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
                    "accenti" => format!("ACCENTI {}", a[0]),
                    "casei" => format!("CASEI {}", a[0]),
                    _ => {
                        if BOOLOPS.contains(&op) {
                            let ref padded = format!(" {} ", op);
                            format!("{}", a.join(padded))
                        } else if SPATIALOPS.contains(&op) {
                            let sop = op.strip_prefix("s_").unwrap();
                            format!("ST_{}({},{})", sop, a[0], a[1])
                        } else if ARRAYOPS.contains(&op) {
                            todo!()

                        } else if TEMPORALOPS.contains(&op) {
                            todo!()
                        } else if CMPOPS.contains(&op) {
                            format!("{} {} {}", a[0], op, a[1])

                        } else if EQOPS.contains(&op) {
                            format!("{} {} {}", a[0], op, a[1])

                        } else if ARITHOPS.contains(&op) {
                            format!("{} {} {}", a[0], op, a[1])

                        } else {
                            todo!()
                        }
                    }
                }
            }

        })
    }
}
