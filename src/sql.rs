use crate::Error;
use crate::Expr;
use crate::Geometry;
use pg_escape::quote_identifier;
use serde_json::{Map as JsonMap, Value as JsonValue};
use sqlparser::ast::DataType::{Date, Timestamp};
use sqlparser::ast::Expr::Value as ValExpr;
use sqlparser::ast::Expr::{Cast, Nested};
use sqlparser::ast::{
    Array as SqlArray, BinaryOperator, CastKind, Expr as SqlExpr, FunctionArgumentList,
    FunctionArguments, Ident, ObjectName, ObjectNamePart, SelectItem, SetExpr, Statement,
    TimezoneInfo, Value,
};
use sqlparser::dialect::PostgreSqlDialect;
use sqlparser::parser::Parser;
use std::fmt;
use std::vec;

/// Identifies whether a name references a function or a property during SQL generation.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum NameKind {
    /// Function identifiers such as `st_intersects`.
    Function,
    /// Property identifiers such as `collection`.
    Property,
}

#[derive(Copy, Clone)]
enum NameResolver<'a> {
    Callback(&'a dyn Fn(&str, NameKind) -> Option<String>),
    Json(&'a JsonMap<String, JsonValue>),
}

/// Options that control how SQL is generated from expressions.
///
/// # Examples
///
/// Mapping properties with a custom callback:
///
/// ```
/// use cql2::{Expr, NameKind, ToSqlAst, ToSqlOptions};
///
/// let expr: Expr = "collection = 'landsat'".parse().unwrap();
/// let resolver = |name: &str, kind: NameKind| match (kind, name) {
///     (NameKind::Property, "collection") => Some("payload ->> 'collection'".to_string()),
///     _ => None,
/// };
///
/// let sql = expr
///     .to_sql_with_options(ToSqlOptions::with_callback(&resolver))
///     .unwrap();
///
/// assert_eq!(sql, "payload ->> 'collection' = 'landsat'");
/// ```
///
/// Using a JSON whitelist for functions and properties:
///
/// ```
/// use cql2::{Expr, ToSqlAst, ToSqlOptions};
/// use serde_json::json;
///
/// let expr: Expr = "casei(name)".parse().unwrap();
/// let mapping = json!({
///     "functions": {"lower": "custom.lower"},
///     "properties": {"collection": "payload ->> 'collection'"}
/// });
/// let map = mapping.as_object().unwrap();
///
/// let sql = expr
///     .to_sql_with_options(ToSqlOptions::with_json(map))
///     .unwrap();
///
/// assert_eq!(sql, "custom.lower(name)");
/// ```
#[derive(Copy, Clone, Default)]
pub struct ToSqlOptions<'a> {
    resolver: Option<NameResolver<'a>>,
}

impl<'a> ToSqlOptions<'a> {
    /// Create a new options value with defaults.
    pub fn new() -> Self {
        Self::default()
    }

    /// Configure a callback resolver that maps function/property names to SQL snippets.
    pub fn with_callback(callback: &'a dyn Fn(&str, NameKind) -> Option<String>) -> Self {
        Self {
            resolver: Some(NameResolver::Callback(callback)),
        }
    }

    /// Configure a JSON resolver containing `functions` and/or `properties` maps.
    pub fn with_json(map: &'a JsonMap<String, JsonValue>) -> Self {
        Self {
            resolver: Some(NameResolver::Json(map)),
        }
    }
}

impl fmt::Debug for ToSqlOptions<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let resolver = match self.resolver {
            Some(NameResolver::Callback(_)) => "Callback",
            Some(NameResolver::Json(_)) => "Json",
            None => "None",
        };
        f.debug_struct("ToSqlOptions")
            .field("resolver", &resolver)
            .finish()
    }
}

/// Trait for converting expressions to SQLParser AST nodes.
pub trait ToSqlAst {
    /// Converts this expression to SQLParser AST.
    fn to_sql_ast(&self) -> Result<SqlExpr, Error> {
        self.to_sql_ast_with_options(ToSqlOptions::default())
    }

    /// Converts this expression to SQLParser AST with custom options.
    fn to_sql_ast_with_options(&self, options: ToSqlOptions<'_>) -> Result<SqlExpr, Error>;

    /// Converts the expression to a SQL string.
    fn to_sql(&self) -> Result<String, Error> {
        self.to_sql_with_options(ToSqlOptions::default())
    }

    /// Converts the expression to a SQL string with custom options.
    fn to_sql_with_options(&self, options: ToSqlOptions<'_>) -> Result<String, Error> {
        let ast = self.to_sql_ast_with_options(options)?;
        Ok(ast.to_string())
    }
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
    func_with_options(name, args, ToSqlOptions::default())
        .unwrap_or_else(|_| panic!("invalid function mapping for {name}"))
}

fn func_with_options(
    name: &str,
    args: Vec<SqlExpr>,
    options: ToSqlOptions<'_>,
) -> Result<SqlExpr, Error> {
    let object_name = function_name(name, options)?;
    Ok(SqlExpr::Function(sqlparser::ast::Function {
        name: object_name,
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
    }))
}

fn lit_expr(value: &str) -> SqlExpr {
    ValExpr(Value::SingleQuotedString(value.to_string()).into())
}
fn float_expr(value: &f64) -> SqlExpr {
    ValExpr(Value::Number(value.to_string(), false).into())
}
fn args2ast(args: &[Box<Expr>], options: ToSqlOptions<'_>) -> Result<Vec<SqlExpr>, Error> {
    args.iter()
        .map(|arg| arg.to_sql_ast_with_options(options))
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

fn lit_or_prop_to_ts(arg: &Expr, options: ToSqlOptions<'_>) -> Result<SqlExpr, Error> {
    match arg {
        Expr::Property { property } => property_expr(property, options),
        Expr::Literal(v) => Ok(cast(
            lit_expr(v),
            Timestamp(None, TimezoneInfo::WithTimeZone),
        )),
        _ => Err(Error::OperationError()),
    }
}

fn lit_or_prop_to_date(arg: &Expr, options: ToSqlOptions<'_>) -> Result<SqlExpr, Error> {
    match arg {
        Expr::Property { property } => property_expr(property, options),
        Expr::Literal(v) => Ok(cast(lit_expr(v), Date)),
        _ => Err(Error::OperationError()),
    }
}

fn t_arg_to_interval(arg: &Expr, options: ToSqlOptions<'_>) -> Result<(SqlExpr, SqlExpr), Error> {
    match arg {
        Expr::Interval { interval } => {
            let start = lit_or_prop_to_ts(&interval[0], options)?;
            let end = lit_or_prop_to_ts(&interval[1], options)?;
            Ok((start, end))
        }
        Expr::Property { property } => {
            let start = property_expr(property, options)?;
            Ok((start.clone(), start))
        }
        Expr::Date { date } => {
            let e = Expr::Date { date: date.clone() };
            let start = e.to_sql_ast_with_options(options)?;
            Ok((start.clone(), start))
        }
        Expr::Timestamp { timestamp } => {
            let e = Expr::Timestamp {
                timestamp: timestamp.clone(),
            };
            let start = e.to_sql_ast_with_options(options)?;
            Ok((start.clone(), start))
        }
        _ => Err(Error::OperationError()),
    }
}

fn t_args(args: &[Box<Expr>], options: ToSqlOptions<'_>) -> Result<Targs, Error> {
    let (left_start, left_end) = t_arg_to_interval(args[0].as_ref(), options)?;
    let (right_start, right_end) = t_arg_to_interval(args[1].as_ref(), options)?;
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

fn property_expr(property: &str, options: ToSqlOptions<'_>) -> Result<SqlExpr, Error> {
    if let Some(mapped) = resolve_name(property, NameKind::Property, options)? {
        parse_sql_expression(&mapped)
    } else {
        Ok(ident(property))
    }
}

fn resolve_name(
    original: &str,
    kind: NameKind,
    options: ToSqlOptions<'_>,
) -> Result<Option<String>, Error> {
    let Some(resolver) = options.resolver else {
        return Ok(None);
    };

    match resolver {
        NameResolver::Callback(callback) => Ok(callback(original, kind)),
        NameResolver::Json(map) => {
            let key = match kind {
                NameKind::Function => "functions",
                NameKind::Property => "properties",
            };

            match map.get(key) {
                Some(JsonValue::Object(section)) => match section.get(original) {
                    Some(JsonValue::String(value)) => Ok(Some(value.clone())),
                    Some(_) => Err(Error::OperationError()),
                    None => Ok(None),
                },
                Some(_) => Err(Error::OperationError()),
                None => Ok(None),
            }
        }
    }
}

fn parse_sql_expression(expr: &str) -> Result<SqlExpr, Error> {
    let dialect = PostgreSqlDialect {};
    let sql = format!("SELECT {expr}");
    let statements = Parser::parse_sql(&dialect, &sql).map_err(|_| Error::OperationError())?;
    if let Some(Statement::Query(query)) = statements.into_iter().next() {
        if let SetExpr::Select(select) = *query.body {
            if let Some(SelectItem::UnnamedExpr(expr)) = select.projection.into_iter().next() {
                return Ok(expr);
            }
        }
    }
    Err(Error::OperationError())
}

fn function_name(name: &str, options: ToSqlOptions<'_>) -> Result<ObjectName, Error> {
    let resolved =
        resolve_name(name, NameKind::Function, options)?.unwrap_or_else(|| name.to_string());
    let parsed = parse_sql_expression(&resolved)?;
    match parsed {
        SqlExpr::Identifier(ident) => Ok(ObjectName(vec![ObjectNamePart::Identifier(ident)])),
        SqlExpr::CompoundIdentifier(idents) => Ok(ObjectName(
            idents.into_iter().map(ObjectNamePart::Identifier).collect(),
        )),
        _ => Err(Error::OperationError()),
    }
}

impl ToSqlAst for Expr {
    fn to_sql_ast_with_options(&self, options: ToSqlOptions<'_>) -> Result<SqlExpr, Error> {
        match self {
            Expr::Bool(v) => Ok(ValExpr(Value::Boolean(*v).into())),
            Expr::Float(v) => Ok(float_expr(v)),
            Expr::Literal(v) => Ok(lit_expr(v)),
            Expr::Date { ref date } => lit_or_prop_to_date(date.as_ref(), options),
            Expr::Timestamp { ref timestamp } => lit_or_prop_to_ts(timestamp.as_ref(), options),
            Expr::Interval { ref interval } => {
                let start = lit_or_prop_to_ts(interval[0].as_ref(), options)?;
                let end = lit_or_prop_to_ts(interval[1].as_ref(), options)?;
                Ok(SqlExpr::Array(SqlArray {
                    elem: vec![start, end],
                    named: true,
                }))
            }
            Expr::Null => Ok(ValExpr(Value::Null.into())),
            Expr::Geometry(v) => match v {
                Geometry::GeoJSON(v) => {
                    let s = lit_expr(&v.to_string());
                    func_with_options("st_geomfromgeojson", vec![s], options)
                }
                Geometry::Wkt(v) => {
                    let s = lit_expr(&v.to_string());
                    func_with_options("st_geomfromtext", vec![s], options)
                }
            },
            Expr::BBox { bbox } => {
                let args = args2ast(bbox, options)?;
                func_with_options("st_makeenvelope", args, options)
            }
            Expr::Array(ref v) => Ok(SqlExpr::Array(SqlArray {
                elem: args2ast(v, options)?,
                named: true,
            })),
            Expr::Property { property } => property_expr(property, options),
            Expr::Operation { op, args } => {
                let op_str = op.to_lowercase();
                let a = args2ast(args, options)?;
                match op_str.as_str() {
                    "isnull" => Ok(SqlExpr::IsNull(Box::new(a[0].clone()))),
                    "not" => Ok(SqlExpr::UnaryOp {
                        op: sqlparser::ast::UnaryOperator::Not,
                        expr: Box::new(a[0].clone()),
                    }),
                    "between" => Ok(SqlExpr::Between {
                        expr: Box::new(a[0].clone()),
                        negated: false,
                        low: Box::new(a[1].clone()),
                        high: Box::new(a[2].clone()),
                    }),
                    "in" => {
                        let expr = a[0].clone();
                        let items = a[1].clone();
                        Ok(SqlExpr::AnyOp {
                            left: Box::new(expr),
                            compare_op: BinaryOperator::Eq,
                            right: Box::new(items),
                            is_some: true,
                        })
                    }
                    "like" => {
                        let expr = a[0].clone();
                        let pattern = a[1].clone();
                        Ok(SqlExpr::Like {
                            expr: Box::new(expr),
                            pattern: Box::new(pattern),
                            escape_char: None,
                            negated: false,
                            any: false,
                        })
                    }
                    "accenti" => func_with_options("strip_accents", a, options),
                    "casei" => func_with_options("lower", a, options),
                    "and" => Ok(andop(a)),
                    "or" => Ok(orop(a)),
                    "=" | "a_equals" | "eq" => Ok(binop(BinaryOperator::Eq, a)),
                    "<>" | "!=" | "ne" => Ok(binop(BinaryOperator::NotEq, a)),
                    ">" | "gt" => Ok(binop(BinaryOperator::Gt, a)),
                    ">=" | "ge" | "gte" => Ok(binop(BinaryOperator::GtEq, a)),
                    "<" | "lt" => Ok(binop(BinaryOperator::Lt, a)),
                    "<=" | "le" | "lte" => Ok(binop(BinaryOperator::LtEq, a)),
                    "+" => Ok(binop(BinaryOperator::Plus, a)),
                    "-" => Ok(binop(BinaryOperator::Minus, a)),
                    "*" => Ok(binop(BinaryOperator::Multiply, a)),
                    "/" => Ok(binop(BinaryOperator::Divide, a)),
                    "%" => Ok(binop(BinaryOperator::Modulo, a)),
                    "^" => func_with_options("power", a, options),
                    "s_intersects" | "st_intersects" | "intersects" => {
                        func_with_options("st_intersects", a, options)
                    }
                    "s_equals" | "st_equals" => func_with_options("st_equals", a, options),
                    "s_within" | "st_within" => func_with_options("st_within", a, options),
                    "s_contains" | "st_contains" => func_with_options("st_contains", a, options),
                    "s_crosses" | "st_crosses" => func_with_options("st_crosses", a, options),
                    "s_overlaps" | "st_overlaps" => func_with_options("st_overlaps", a, options),
                    "s_touches" | "st_touches" => func_with_options("st_touches", a, options),
                    "s_disjoint" | "st_disjoint" => func_with_options("st_disjoint", a, options),
                    "a_contains" => Ok(binop(BinaryOperator::AtArrow, a)),
                    "a_containedby" => Ok(binop(BinaryOperator::ArrowAt, a)),
                    "a_overlaps" => Ok(binop(BinaryOperator::AtAt, a)),
                    "t_before" => {
                        let t = t_args(args, options)?;
                        Ok(ltop(t.left_end, t.right_start))
                    }
                    "t_after" => {
                        let t = t_args(args, options)?;
                        Ok(ltop(t.right_end, t.left_start))
                    }
                    "t_meets" => {
                        let t = t_args(args, options)?;
                        Ok(eqop(t.left_end, t.right_start))
                    }
                    "t_metby" => {
                        let t = t_args(args, options)?;
                        Ok(eqop(t.right_end, t.left_start))
                    }
                    "t_overlaps" => {
                        let t = t_args(args, options)?;
                        Ok(wrap(andop(vec![
                            ltop(t.left_start, t.right_end.clone()),
                            ltop(t.right_start, t.left_end.clone()),
                            ltop(t.left_end, t.right_end),
                        ])))
                    }
                    "t_overlappedby" => {
                        let t = t_args(args, options)?;
                        Ok(wrap(andop(vec![
                            ltop(t.right_start, t.left_end.clone()),
                            ltop(t.left_start, t.right_end.clone()),
                            ltop(t.right_end, t.left_end),
                        ])))
                    }
                    "t_starts" => {
                        let t = t_args(args, options)?;
                        Ok(wrap(andop(vec![
                            eqop(t.left_start, t.right_start.clone()),
                            ltop(t.left_end, t.right_end),
                        ])))
                    }
                    "t_startedby" => {
                        let t = t_args(args, options)?;
                        Ok(wrap(andop(vec![
                            eqop(t.right_start, t.left_start.clone()),
                            ltop(t.right_end, t.left_end),
                        ])))
                    }
                    "t_during" => {
                        let t = t_args(args, options)?;
                        Ok(wrap(andop(vec![
                            gtop(t.left_start, t.right_start),
                            ltop(t.left_end, t.right_end),
                        ])))
                    }
                    "t_contains" => {
                        let t = t_args(args, options)?;
                        Ok(wrap(andop(vec![
                            gtop(t.right_start, t.left_start),
                            ltop(t.right_end, t.left_end),
                        ])))
                    }
                    "t_finishes" => {
                        let t = t_args(args, options)?;
                        Ok(wrap(andop(vec![
                            eqop(t.left_end, t.right_end),
                            gtop(t.left_start, t.right_start),
                        ])))
                    }
                    "t_finishedby" => {
                        let t = t_args(args, options)?;
                        Ok(wrap(andop(vec![
                            eqop(t.right_end, t.left_end),
                            gtop(t.right_start, t.left_start),
                        ])))
                    }
                    "t_equals" => {
                        let t = t_args(args, options)?;
                        Ok(wrap(andop(vec![
                            eqop(t.left_start, t.right_start),
                            eqop(t.left_end, t.right_end),
                        ])))
                    }
                    "t_disjoint" => {
                        let t = t_args(args, options)?;
                        Ok(notop(wrap(andop(vec![
                            lteop(t.left_start, t.right_end),
                            gteop(t.left_end, t.right_start),
                        ]))))
                    }
                    "t_intersects" | "anyinteracts" => {
                        let t = t_args(args, options)?;
                        Ok(wrap(andop(vec![
                            lteop(t.left_start, t.right_end),
                            gteop(t.left_end, t.right_start),
                        ])))
                    }
                    _ => func_with_options(&op_str, a, options),
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{NameKind, ToSqlAst, ToSqlOptions};
    use crate::Expr;
    use serde_json::json;

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

    #[test]
    fn test_property_resolver_callback() {
        let expr: Expr = "collection = 'landsat'".parse().unwrap();
        let resolver = |name: &str, kind: NameKind| match (kind, name) {
            (NameKind::Property, "collection") => Some("payload ->> 'collection'".to_string()),
            _ => None,
        };
        let sql = expr
            .to_sql_with_options(ToSqlOptions::with_callback(&resolver))
            .unwrap();
        assert_eq!(sql, "payload ->> 'collection' = 'landsat'");
    }

    #[test]
    fn test_property_resolver_json() {
        let mapping = json!({
            "properties": {"collection": "payload ->> 'collection'"}
        });
        let map = mapping.as_object().unwrap();
        let expr: Expr = "collection = 'landsat'".parse().unwrap();
        let sql = expr
            .to_sql_with_options(ToSqlOptions::with_json(map))
            .unwrap();
        assert_eq!(sql, "payload ->> 'collection' = 'landsat'");
    }

    #[test]
    fn test_function_resolver_json() {
        let mapping = json!({
            "functions": {"lower": "custom.lower"}
        });
        let map = mapping.as_object().unwrap();
        let expr: Expr = "casei(name)".parse().unwrap();
        let sql = expr
            .to_sql_with_options(ToSqlOptions::with_json(map))
            .unwrap();
        assert_eq!(sql, "custom.lower(name)");
    }
}
