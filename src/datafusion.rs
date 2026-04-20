use crate::{sql::func, Error, Expr, Geometry, ToSqlAst};
use sqlparser::ast::{
    visit_expressions_mut, BinaryOperator, DataType, Expr as SqlExpr, FunctionArg, FunctionArgExpr,
    FunctionArguments, TimezoneInfo, Value,
};
use std::ops::ControlFlow;

/// Traits for generating SQL compatible with DataFusion and geodatafusion.
pub trait ToDataFusionSQL {
    /// Convert Expression to SQL for DataFusion and geodatafusion.
    fn to_datafusion_sql(&self) -> Result<String, Error>;
}

impl ToDataFusionSQL for Expr {
    /// Converts this expression to DataFusion-compatible SQL.
    fn to_datafusion_sql(&self) -> Result<String, Error> {
        let mut ast = self.to_sql_ast()?;
        let mut rewrite_error = None;

        let _ = visit_expressions_mut(&mut ast, |expr| {
            if let Err(err) = rewrite_expr(expr) {
                rewrite_error = Some(err);
                return ControlFlow::<()>::Break(());
            }
            ControlFlow::<()>::Continue(())
        });

        if let Some(err) = rewrite_error {
            return Err(err);
        }

        Ok(ast.to_string())
    }
}

fn rewrite_expr(expr: &mut SqlExpr) -> Result<(), Error> {
    match expr {
        SqlExpr::BinaryOp { op, left, right } => match *op {
            BinaryOperator::AtArrow => {
                *expr = func("array_has_all", vec![*left.clone(), *right.clone()]);
            }
            BinaryOperator::ArrowAt => {
                *expr = func("array_has_all", vec![*right.clone(), *left.clone()]);
            }
            BinaryOperator::AtAt => {
                *expr = func("array_has_any", vec![*left.clone(), *right.clone()]);
            }
            _ => {}
        },
        SqlExpr::Cast {
            expr: inner,
            data_type,
            ..
        } => match data_type {
            DataType::Date => {
                if let Some(value) = as_string_literal(inner) {
                    *expr = func(
                        "to_timestamp",
                        vec![string_literal(&format!("{}T00:00:00Z", value))],
                    );
                } else {
                    *expr = func("to_timestamp", vec![*inner.clone()]);
                }
            }
            DataType::Timestamp(_, TimezoneInfo::WithTimeZone) => {
                *expr = func("to_timestamp", vec![*inner.clone()]);
            }
            _ => {}
        },
        SqlExpr::AnyOp {
            left,
            compare_op: BinaryOperator::Eq,
            right,
            is_some,
        } => {
            if let SqlExpr::Array(array) = right.as_ref() {
                *expr = SqlExpr::InList {
                    expr: left.clone(),
                    list: array.elem.clone(),
                    negated: !*is_some,
                };
            }
        }
        SqlExpr::Function(function)
            if function_name(function.name.to_string().as_str(), "st_geomfromgeojson") =>
        {
            let wkt = function
                .args
                .clone()
                .into_list()
                .and_then(extract_single_string_literal)
                .map(geojson_to_wkt)
                .transpose()?;
            if let Some(wkt) = wkt {
                *expr = func("st_geomfromtext", vec![string_literal(&wkt)]);
            }
        }
        _ => {}
    }
    Ok(())
}

fn function_name(actual: &str, expected: &str) -> bool {
    actual.eq_ignore_ascii_case(expected)
}

fn string_literal(value: &str) -> SqlExpr {
    SqlExpr::Value(Value::SingleQuotedString(value.to_string()).into())
}

fn extract_single_string_literal(args: Vec<FunctionArg>) -> Option<String> {
    if args.len() != 1 {
        return None;
    }
    match &args[0] {
        FunctionArg::Unnamed(FunctionArgExpr::Expr(SqlExpr::Value(value))) => match &value.value {
            Value::SingleQuotedString(string) | Value::DoubleQuotedString(string) => {
                Some(string.clone())
            }
            _ => None,
        },
        _ => None,
    }
}

fn as_string_literal(expr: &SqlExpr) -> Option<String> {
    match expr {
        SqlExpr::Value(value) => match &value.value {
            Value::SingleQuotedString(string) | Value::DoubleQuotedString(string) => {
                Some(string.clone())
            }
            _ => None,
        },
        _ => None,
    }
}

fn geojson_to_wkt(geojson: String) -> Result<String, Error> {
    let geometry: geojson::Geometry = serde_json::from_str(&geojson)?;
    let geometry = Geometry::GeoJSON(geometry);
    geometry.to_wkt()
}

trait FunctionArgumentsExt {
    fn into_list(self) -> Option<Vec<FunctionArg>>;
}

impl FunctionArgumentsExt for FunctionArguments {
    fn into_list(self) -> Option<Vec<FunctionArg>> {
        match self {
            FunctionArguments::List(list) => Some(list.args),
            FunctionArguments::None => None,
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ToDataFusionSQL;
    use crate::{parse_json, Expr};

    #[test]
    fn test_to_datafusion_sql() {
        let expr: Expr = "foo = 1".parse().unwrap();
        assert_eq!(expr.to_datafusion_sql().unwrap(), "foo = 1");
    }

    #[test]
    fn test_array_ops() {
        let expr: Expr = "a_contains(foo, bar)".parse().unwrap();
        assert_eq!(expr.to_datafusion_sql().unwrap(), "array_has_all(foo, bar)");
    }

    #[test]
    fn test_date_and_timestamp_literals() {
        let expr: Expr = "DATE('2020-01-01') < d and TIMESTAMP('2020-01-01T00:00:00Z') <= t"
            .parse()
            .unwrap();
        assert_eq!(
            expr.to_datafusion_sql().unwrap(),
            "to_timestamp('2020-01-01T00:00:00Z') < d AND to_timestamp('2020-01-01T00:00:00Z') <= t"
        );
    }

    #[test]
    fn test_in_list() {
        let expr: Expr = "in(foo, ('bar', 'baz'))".parse().unwrap();
        assert_eq!(expr.to_datafusion_sql().unwrap(), "foo IN ('bar', 'baz')");
    }

    #[test]
    fn test_geojson_literal_rewritten_to_wkt() {
        let expr = parse_json(
            r#"{"op":"s_intersects","args":[{"property":"geom"},{"type":"Point","coordinates":[0.0,1.0]}]}"#,
        )
        .unwrap();
        assert_eq!(
            expr.to_datafusion_sql().unwrap(),
            "st_intersects(geom, st_geomfromtext('POINT(0 1)'))"
        );
    }
}
