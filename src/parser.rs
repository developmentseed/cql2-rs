use crate::{Error, Expr, Geometry};
use pest::{
    iterators::{Pair, Pairs},
    pratt_parser::PrattParser,
    Parser,
};

/// Parses a cql2-text string into a CQL2 expression.
///
/// # Examples
///
/// ```
/// let s = "landsat:scene_id = 'LC82030282019133LGN00'";
/// let expr = cql2::parse_text(s);
/// ```
pub fn parse_text(s: &str) -> Result<Expr, Error> {
    let mut pairs = CQL2Parser::parse(Rule::Expr, s).map_err(Box::new)?;
    if let Some(pair) = pairs.next() {
        if pairs.next().is_some() {
            Err(Error::InvalidCql2Text(s.to_string()))
        } else {
            parse_expr(pair.into_inner())
        }
    } else {
        Err(Error::InvalidCql2Text(s.to_string()))
    }
}

#[derive(pest_derive::Parser)]
#[grammar = "cql2.pest"]
struct CQL2Parser;

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
    let op = op.to_lowercase();
    if op == "eq" {
        "=".to_string()
    } else {
        op
    }
}

fn strip_quotes(s: &str) -> &str {
    if (s.starts_with('"') && s.ends_with('"')) || (s.starts_with('\'') && s.ends_with('\'')) {
        &s[1..s.len() - 1]
    } else {
        s
    }
}

fn opstr(op: Pair<'_, Rule>) -> String {
    normalize_op(op.as_str())
}

fn parse_expr(expression_pairs: Pairs<'_, Rule>) -> Result<Expr, Error> {
    PRATT_PARSER
        .map_primary(|primary| match primary.as_rule() {
            Rule::Expr | Rule::ExpressionInParentheses => parse_expr(primary.into_inner()),
            Rule::Unsigned => Ok(Expr::Float(primary.as_str().parse::<f64>()?)),
            Rule::DECIMAL => Ok(Expr::Float(primary.as_str().parse::<f64>()?)),
            Rule::SingleQuotedString => {
                Ok(Expr::Literal(strip_quotes(primary.as_str()).to_string()))
            }
            Rule::True | Rule::False => {
                let bool_value = primary.as_str().to_lowercase().parse::<bool>()?;
                Ok(Expr::Bool(bool_value))
            }
            Rule::Identifier => Ok(Expr::Property {
                property: strip_quotes(primary.as_str()).to_string(),
            }),
            Rule::GEOMETRY => {
                // These are some incredibly annoying backflips to handle
                // geometries without `Z` but that have 3D coordinates. It's
                // not part of OGC WKT, but CQL2 demands 🤦‍♀️.
                let start = primary.as_span().start();
                let s = primary.as_str().to_string();
                let pairs = primary.into_inner();
                if pairs.find_first_tagged("three_d").is_some() {
                    let zm = pairs
                        .flatten()
                        .find(|pair| matches!(pair.as_rule(), Rule::ZM))
                        .expect("all geometries should have a ZM rule");
                    if zm.as_str().chars().all(|c| c.is_ascii_whitespace()) {
                        let span = zm.as_span();
                        let s = format!(
                            "{} Z{}",
                            &s[0..span.start() - start],
                            &s[span.end() - start..]
                        );
                        return Ok(Expr::Geometry(Geometry::Wkt(s)));
                    }
                }
                Ok(Expr::Geometry(Geometry::Wkt(s)))
            }
            Rule::Function => {
                let mut pairs = primary.into_inner();
                let op = strip_quotes(
                    pairs
                        .next()
                        .expect("the grammar guarantees that there is always an op")
                        .as_str(),
                )
                .to_lowercase();
                let mut args = Vec::new();
                for pair in pairs {
                    args.push(Box::new(parse_expr(pair.into_inner())?))
                }
                match op.as_str() {
                    "interval" => Ok(Expr::Interval { interval: args }),
                    "date" => Ok(Expr::Date {
                        date: args
                            .into_iter()
                            .next()
                            .ok_or(Error::MissingArgument("date"))?,
                    }),
                    "timestamp" => Ok(Expr::Timestamp {
                        timestamp: args
                            .into_iter()
                            .next()
                            .ok_or(Error::MissingArgument("timestamp"))?,
                    }),
                    _ => Ok(Expr::Operation { op, args }),
                }
            }
            Rule::Array => {
                let pairs = primary.into_inner();
                let mut array_elements = Vec::new();
                for pair in pairs {
                    array_elements.push(Box::new(parse_expr(pair.into_inner())?))
                }
                Ok(Expr::Array(array_elements))
            }

            rule => unreachable!("Expr::parse expected atomic rule, found {:?}", rule),
        })
        .map_infix(|lhs, op, rhs| {
            let lhs = lhs?;
            let rhs = rhs?;
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
                    return Ok(retexpr);
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

                return Ok(Expr::Operation {
                    op: "and".to_string(),
                    args: andargs,
                });
            } else {
                let mut outargs: Vec<Box<Expr>> = Vec::new();

                match lhsclone {
                    Expr::Operation { ref op, ref args } if op == "and" && op == &opstring => {
                        for arg in args.iter() {
                            outargs.push(arg.clone());
                        }
                        outargs.push(Box::new(rhsclone));
                        return Ok(Expr::Operation {
                            op: opstring,
                            args: outargs,
                        });
                    }
                    _ => (),
                }
                retexpr = Expr::Operation {
                    op: opstring,
                    args: origargs,
                };
            }

            if notflag {
                return Ok(Expr::Operation {
                    op: "not".to_string(),
                    args: vec![Box::new(retexpr)],
                });
            }
            Ok(retexpr)
        })
        .map_prefix(|op, child| {
            let child = child?;
            match op.as_rule() {
                Rule::UnaryNot => Ok(Expr::Operation {
                    op: "not".to_string(),
                    args: vec![Box::new(child)],
                }),
                Rule::Negative => Ok(Expr::Operation {
                    op: "*".to_string(),
                    args: vec![Box::new(Expr::Float(-1.0)), Box::new(child)],
                }),
                rule => unreachable!("Expr::parse expected prefix operator, found {:?}", rule),
            }
        })
        .map_postfix(|child, op| {
            let child = child?;
            let notflag = &op.clone().into_inner().next().is_some();
            let retexpr = match op.as_rule() {
                Rule::IsNullPostfix => Expr::Operation {
                    op: "isNull".to_string(),
                    args: vec![Box::new(child)],
                },
                rule => unreachable!("Expr::parse expected postfix operator, found {:?}", rule),
            };
            if *notflag {
                return Ok(Expr::Operation {
                    op: "not".to_string(),
                    args: vec![Box::new(retexpr)],
                });
            };
            Ok(retexpr)
        })
        .parse(expression_pairs)
}

#[cfg(test)]
mod tests {
    use super::{CQL2Parser, Rule};
    use pest::Parser;

    #[test]
    fn point_zm() {
        let _ = CQL2Parser::parse(Rule::GEOMETRY, "POINT ZM(-105.1019 40.1672 4981 42)").unwrap();
    }
}
