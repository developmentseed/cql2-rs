use crate::Error;
use crate::Expr;
use crate::Geometry;
use pg_escape::quote_identifier;
use sqlparser::ast::DataType::{Date, Timestamp};
use sqlparser::ast::Expr::Value as ValExpr;
use sqlparser::ast::Expr::{Cast, Nested};
use sqlparser::ast::{
    Array as SqlArray, BinaryOperator, CastKind, Expr as SqlExpr, FunctionArgumentList,
    FunctionArguments, Ident, TimezoneInfo, Value,
};
use std::vec;

/// Trait for converting expressions to SQLParser AST nodes.
pub trait ToSqlAst {
    /// Converts this expression to SQLParser AST.
    fn to_sql_ast(&self) -> Result<SqlExpr, Error>;
    /// Converts the expression to a SQL string.
    fn to_sql(&self) -> Result<String, Error>;
}

fn cast(arg: SqlExpr, data_type: sqlparser::ast::DataType) -> SqlExpr {
    Cast {
        expr: Box::new(arg),
        data_type,
        kind: CastKind::Cast,
        format: None,
    }
}

pub(crate) fn func(name: &str, args: Vec<SqlExpr>) -> SqlExpr {
    SqlExpr::Function(sqlparser::ast::Function {
        name: sqlparser::ast::ObjectName(vec![sqlparser::ast::ObjectNamePart::Identifier(
            ident_inner(name),
        )]),
        args: FunctionArguments::List(FunctionArgumentList {
            duplicate_treatment: None,
            args: args
                .into_iter()
                .map(|arg| {
                    sqlparser::ast::FunctionArg::Unnamed(sqlparser::ast::FunctionArgExpr::Expr(arg))
                })
                .collect(),
            clauses: vec![],
        }),
        over: None,
        filter: None,
        null_treatment: None,
        within_group: vec![],
        uses_odbc_syntax: false,
        parameters: FunctionArguments::None,
    })
}

fn lit_expr(value: &str) -> SqlExpr {
    ValExpr(Value::SingleQuotedString(value.to_string()).into())
}
fn float_expr(value: &f64) -> SqlExpr {
    ValExpr(Value::Number(value.to_string(), false).into())
}
fn args2ast(args: &[Box<Expr>]) -> Result<Vec<SqlExpr>, Error> {
    args.iter()
        .map(|arg| arg.to_sql_ast())
        .collect::<Result<Vec<_>, _>>()
}
fn binop(op: BinaryOperator, args: Vec<SqlExpr>) -> SqlExpr {
    SqlExpr::BinaryOp {
        left: Box::new(args[0].clone()),
        op,
        right: Box::new(args[1].clone()),
    }
}

struct Targs {
    left_start: SqlExpr,
    left_end: SqlExpr,
    right_start: SqlExpr,
    right_end: SqlExpr,
}

fn lit_or_prop_to_ts(arg: &Expr) -> Result<SqlExpr, Error> {
    Ok(match arg {
        Expr::Property { property } => ident(property),
        Expr::Literal(v) => cast(lit_expr(v), Timestamp(None, TimezoneInfo::WithTimeZone)),
        _ => return Err(Error::OperationError()),
    })
}

fn lit_or_prop_to_date(arg: &Expr) -> Result<SqlExpr, Error> {
    Ok(match arg {
        Expr::Property { property } => ident(property),
        Expr::Literal(v) => cast(lit_expr(v), Date),
        _ => return Err(Error::OperationError()),
    })
}

fn t_arg_to_interval(arg: &Expr) -> Result<(SqlExpr, SqlExpr), Error> {
    match arg {
        Expr::Interval { interval } => {
            let start = lit_or_prop_to_ts(&interval[0])?;
            let end = lit_or_prop_to_ts(&interval[1])?;
            Ok((start, end))
        }
        Expr::Property { property } => {
            let start = ident(property);
            Ok((start.clone(), start.clone()))
        }
        Expr::Date { date } => {
            let e = Expr::Date { date: date.clone() };
            let start = e.to_sql_ast()?;
            Ok((start.clone(), start.clone()))
        }
        Expr::Timestamp { timestamp } => {
            let e = Expr::Timestamp {
                timestamp: timestamp.clone(),
            };
            let start = e.to_sql_ast()?;
            Ok((start.clone(), start.clone()))
        }
        _ => Err(Error::OperationError()),
    }
}

fn t_args(args: &[Box<Expr>]) -> Result<Targs, Error> {
    let (left_start, left_end) = t_arg_to_interval(args[0].as_ref())?;
    let (right_start, right_end) = t_arg_to_interval(args[1].as_ref())?;
    Ok(Targs {
        left_start,
        left_end,
        right_start,
        right_end,
    })
}

fn andop(args: Vec<SqlExpr>) -> SqlExpr {
    args.into_iter()
        .reduce(|left, right| SqlExpr::BinaryOp {
            left: Box::new(left),
            op: BinaryOperator::And,
            right: Box::new(right),
        })
        .expect("andop requires at least one argument")
}

fn orop(args: Vec<SqlExpr>) -> SqlExpr {
    args.into_iter()
        .reduce(|left, right| SqlExpr::BinaryOp {
            left: Box::new(left),
            op: BinaryOperator::Or,
            right: Box::new(right),
        })
        .expect("orop requires at least one argument")
}

fn ltop(left: SqlExpr, right: SqlExpr) -> SqlExpr {
    SqlExpr::BinaryOp {
        left: Box::new(left),
        op: BinaryOperator::Lt,
        right: Box::new(right),
    }
}

fn gtop(left: SqlExpr, right: SqlExpr) -> SqlExpr {
    SqlExpr::BinaryOp {
        left: Box::new(left),
        op: BinaryOperator::Gt,
        right: Box::new(right),
    }
}

fn lteop(left: SqlExpr, right: SqlExpr) -> SqlExpr {
    SqlExpr::BinaryOp {
        left: Box::new(left),
        op: BinaryOperator::LtEq,
        right: Box::new(right),
    }
}

fn gteop(left: SqlExpr, right: SqlExpr) -> SqlExpr {
    SqlExpr::BinaryOp {
        left: Box::new(left),
        op: BinaryOperator::GtEq,
        right: Box::new(right),
    }
}

fn eqop(left: SqlExpr, right: SqlExpr) -> SqlExpr {
    SqlExpr::BinaryOp {
        left: Box::new(left),
        op: BinaryOperator::Eq,
        right: Box::new(right),
    }
}

fn notop(arg: SqlExpr) -> SqlExpr {
    SqlExpr::UnaryOp {
        op: sqlparser::ast::UnaryOperator::Not,
        expr: Box::new(arg),
    }
}

fn wrap(arg: SqlExpr) -> SqlExpr {
    Nested(Box::new(arg))
}

fn ident_inner(property: &str) -> Ident {
    let p = quote_identifier(property);
    if p.starts_with('"') && p.ends_with('"') {
        Ident::with_quote('"', p[1..p.len() - 1].to_string())
    } else {
        Ident::new(p)
    }
}

fn ident(property: &str) -> SqlExpr {
    SqlExpr::Identifier(ident_inner(property))
}

impl ToSqlAst for Expr {
    /// Converts this expression to SQLParser AST.
    fn to_sql_ast(&self) -> Result<SqlExpr, Error> {
        Ok(match self {
            Expr::Bool(v) => ValExpr(Value::Boolean(*v).into()),
            Expr::Float(v) => float_expr(v),
            Expr::Literal(v) => lit_expr(v),
            Expr::Date { ref date } => lit_or_prop_to_date(date.as_ref())?,
            Expr::Timestamp { ref timestamp } => lit_or_prop_to_ts(timestamp.as_ref())?,
            Expr::Interval { ref interval } => {
                let start = lit_or_prop_to_ts(interval[0].as_ref())?;
                let end = lit_or_prop_to_ts(interval[1].as_ref())?;
                SqlExpr::Array(SqlArray {
                    elem: vec![start, end],
                    named: true,
                })
            }
            Expr::Null => ValExpr(Value::Null.into()),
            Expr::Geometry(v) => match v {
                Geometry::GeoJSON(v) => {
                    let s = lit_expr(&v.to_string());
                    func("st_geomfromgeojson", vec![s])
                }
                Geometry::Wkt(v) => {
                    let s = lit_expr(&v.to_string());
                    func("st_geomfromtext", vec![s])
                }
            },

            Expr::BBox { bbox } => func("st_makeenvelope", args2ast(bbox)?),
            Expr::Array(ref v) => SqlExpr::Array(SqlArray {
                elem: args2ast(v)?,
                named: true,
            }),
            Expr::Property { property } => ident(property),
            Expr::Operation { op, args } => {
                let op_str = op.to_lowercase();
                let a = args2ast(args)?;
                match op_str.as_str() {
                    "isnull" => SqlExpr::IsNull(Box::new(a[0].clone())),
                    "not" => SqlExpr::UnaryOp {
                        op: sqlparser::ast::UnaryOperator::Not,
                        expr: Box::new(a[0].clone()),
                    },
                    "between" => SqlExpr::Between {
                        expr: Box::new(a[0].clone()),
                        negated: false,
                        low: Box::new(a[1].clone()),
                        high: Box::new(a[2].clone()),
                    },
                    "in" => {
                        let expr = a[0].clone();
                        let items = a[1].clone();
                        SqlExpr::AnyOp {
                            left: Box::new(expr),
                            compare_op: BinaryOperator::Eq,
                            right: Box::new(items),
                            is_some: true,
                        }
                    }
                    "like" => {
                        let expr = a[0].clone();
                        let pattern = a[1].clone();
                        SqlExpr::Like {
                            expr: Box::new(expr),
                            pattern: Box::new(pattern),
                            escape_char: None,
                            negated: false,
                            any: false,
                        }
                    }
                    "accenti" => func("strip_accents", a),
                    "casei" => func("lower", a),
                    "and" => andop(a),
                    "or" => orop(a),
                    "=" | "a_equals" | "eq" => binop(BinaryOperator::Eq, a),
                    "<>" | "!=" | "ne" => binop(BinaryOperator::NotEq, a),
                    ">" | "gt" => binop(BinaryOperator::Gt, a),
                    ">=" | "ge" | "gte" => binop(BinaryOperator::GtEq, a),
                    "<" | "lt" => binop(BinaryOperator::Lt, a),
                    "<=" | "le" | "lte" => binop(BinaryOperator::LtEq, a),
                    "+" => binop(BinaryOperator::Plus, a),
                    "-" => binop(BinaryOperator::Minus, a),
                    "*" => binop(BinaryOperator::Multiply, a),
                    "/" => binop(BinaryOperator::Divide, a),
                    "%" => binop(BinaryOperator::Modulo, a),
                    "^" => func("power", a),
                    "s_intersects" | "st_intersects" | "intersects" => func("st_intersects", a),
                    "s_equals" | "st_equals" => func("st_equals", a),
                    "s_within" | "st_within" => func("st_within", a),
                    "s_contains" | "st_contains" => func("st_contains", a),
                    "s_crosses" | "st_crosses" => func("st_crosses", a),
                    "s_overlaps" | "st_overlaps" => func("st_overlaps", a),
                    "s_touches" | "st_touches" => func("st_touches", a),
                    "s_disjoint" | "st_disjoint" => func("st_disjoint", a),
                    "a_contains" => binop(BinaryOperator::AtArrow, a),
                    "a_containedby" => binop(BinaryOperator::ArrowAt, a),
                    "a_overlaps" => binop(BinaryOperator::AtAt, a),
                    "t_before" => {
                        let t = t_args(args)?;
                        ltop(t.left_end, t.right_start)
                    }
                    "t_after" => {
                        let t = t_args(args)?;
                        ltop(t.right_end, t.left_start)
                    }
                    "t_meets" => {
                        let t = t_args(args)?;
                        eqop(t.left_end, t.right_start)
                    }
                    "t_metby" => {
                        let t = t_args(args)?;
                        eqop(t.right_end, t.left_start)
                    }
                    "t_overlaps" => {
                        let t = t_args(args)?;
                        wrap(andop(vec![
                            ltop(t.left_start, t.right_end.clone()),
                            ltop(t.right_start, t.left_end.clone()),
                            ltop(t.left_end, t.right_end),
                        ]))
                    }
                    "t_overlappedby" => {
                        let t = t_args(args)?;
                        wrap(andop(vec![
                            ltop(t.right_start, t.left_end.clone()),
                            ltop(t.left_start, t.right_end.clone()),
                            ltop(t.right_end, t.left_end),
                        ]))
                    }
                    "t_starts" => {
                        let t = t_args(args)?;
                        wrap(andop(vec![
                            eqop(t.left_start, t.right_start.clone()),
                            ltop(t.left_end, t.right_end),
                        ]))
                    }
                    "t_startedby" => {
                        let t = t_args(args)?;
                        wrap(andop(vec![
                            eqop(t.right_start, t.left_start.clone()),
                            ltop(t.right_end, t.left_end),
                        ]))
                    }
                    "t_during" => {
                        let t = t_args(args)?;
                        wrap(andop(vec![
                            gtop(t.left_start, t.right_start),
                            ltop(t.left_end, t.right_end),
                        ]))
                    }
                    "t_contains" => {
                        let t = t_args(args)?;
                        wrap(andop(vec![
                            gtop(t.right_start, t.left_start),
                            ltop(t.right_end, t.left_end),
                        ]))
                    }
                    "t_finishes" => {
                        let t = t_args(args)?;
                        wrap(andop(vec![
                            eqop(t.left_end, t.right_end),
                            gtop(t.left_start, t.right_start),
                        ]))
                    }
                    "t_finishedby" => {
                        let t = t_args(args)?;
                        wrap(andop(vec![
                            eqop(t.right_end, t.left_end),
                            gtop(t.right_start, t.left_start),
                        ]))
                    }
                    "t_equals" => {
                        let t = t_args(args)?;
                        wrap(andop(vec![
                            eqop(t.left_start, t.right_start),
                            eqop(t.left_end, t.right_end),
                        ]))
                    }
                    "t_disjoint" => {
                        let t = t_args(args)?;
                        notop(wrap(andop(vec![
                            lteop(t.left_start, t.right_end),
                            gteop(t.left_end, t.right_start),
                        ])))
                    }
                    "t_intersects" | "anyinteracts" => {
                        let t = t_args(args)?;
                        wrap(andop(vec![
                            lteop(t.left_start, t.right_end),
                            gteop(t.left_end, t.right_start),
                        ]))
                    }
                    _ => func(&op_str, a),
                }
            }
        })
    }

    /// Converts the expression to a SQL string.
    fn to_sql(&self) -> Result<String, Error> {
        let ast = self.to_sql_ast()?;
        Ok(ast.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::ToSqlAst;
    use crate::Expr;

    #[test]
    fn test_basic_expression() {
        let expr: Expr = "1 + 2 > 4".parse().unwrap();
        let sql_ast = expr.to_sql_ast().unwrap();
        let sql_str = sql_ast.to_string();
        assert_eq!(sql_str, "1 + 2 > 4");
    }

    #[test]
    fn test_t_before_expression() {
        // t_before([start1, end1], [start2, end2]) => end1 < start2
        let expr: Expr = "t_before(ts_start, DATE('2020-02-01'))".parse().unwrap();
        let sql_ast = expr.to_sql_ast().expect("to_sql_ast failed");
        let sql_str = sql_ast.to_string();
        assert_eq!(sql_str, "ts_start < CAST('2020-02-01' AS DATE)");
    }
}
