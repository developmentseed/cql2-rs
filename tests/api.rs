//! Conversions and helpers of the public API that the other tests do not reach.

use cql2::{Error, Expr, Validator};
use geo::BoundingRect;
use geo_types::Geometry as GeoGeometry;
use serde_json::{json, Value};
use std::collections::HashSet;

#[test]
fn an_expression_converts_to_a_number_from_a_number_or_a_numeric_string() {
    assert_eq!(f64::try_from(Expr::Float(2.5)).unwrap(), 2.5);
    assert_eq!(
        f64::try_from(Expr::Literal("-1e3".into())).unwrap(),
        -1000.0
    );
    assert!(matches!(
        f64::try_from(Expr::Literal("x".into())),
        Err(Error::ParseFloat(_))
    ));
    assert!(matches!(
        f64::try_from(Expr::Bool(true)),
        Err(Error::ExprToF64(_))
    ));
}

#[test]
fn an_expression_converts_to_a_string_from_a_scalar() {
    assert_eq!(String::try_from(Expr::Literal("x".into())).unwrap(), "x");
    assert_eq!(String::try_from(Expr::Bool(false)).unwrap(), "false");
    assert_eq!(String::try_from(Expr::Float(3.0)).unwrap(), "3");
    assert!(String::try_from(Expr::Null).is_err());
}

#[test]
fn an_array_converts_to_the_set_of_its_items_as_cql2_text() {
    let array: Expr = "('x', 1, 'x')".parse().unwrap();
    let set = HashSet::<String>::try_from(array).unwrap();
    assert_eq!(set, HashSet::from(["'x'".to_string(), "1".to_string()]));
    assert!(HashSet::<String>::try_from(Expr::Literal("x".into())).is_err());
}

#[test]
fn a_geometry_or_a_bbox_converts_to_geo_types() {
    let point: Expr = "POINT(1 2)".parse().unwrap();
    assert_eq!(
        GeoGeometry::try_from(point).unwrap(),
        GeoGeometry::Point((1.0, 2.0).into())
    );
    let bbox: Expr = "BBOX(0, 0, 1, 2)".parse().unwrap();
    let rect = GeoGeometry::try_from(bbox)
        .unwrap()
        .bounding_rect()
        .unwrap();
    assert_eq!(
        (rect.min(), rect.max()),
        ((0.0, 0.0).into(), (1.0, 2.0).into())
    );
    assert!(matches!(
        GeoGeometry::try_from(Expr::Float(1.0)),
        Err(Error::ExprToGeom(_))
    ));
}

#[test]
fn an_expression_converts_to_its_cql2_json_value() {
    let expr: Expr = "a = 1".parse().unwrap();
    assert_eq!(
        Value::try_from(expr).unwrap(),
        json!({"op": "=", "args": [{"property": "a"}, 1.0]})
    );
}

#[test]
fn pretty_json_parses_back() {
    let expr: Expr = "a = 1 AND s_intersects(g, POINT(1 2))".parse().unwrap();
    let pretty = expr.to_json_pretty().unwrap();
    assert!(pretty.contains('\n'));
    assert_eq!(cql2::parse_json(&pretty).unwrap(), expr);
}

#[test]
fn adding_expressions_joins_them_with_and() {
    let a: Expr = "a = 1".parse().unwrap();
    let b: Expr = "b = 2".parse().unwrap();
    assert_eq!(a + b, "a = 1 AND b = 2".parse().unwrap());
}

#[test]
fn parse_file_reads_either_encoding() {
    let text = cql2::parse_file("examples/text/example01.txt").unwrap();
    let json = cql2::parse_file("examples/json/example01.json").unwrap();
    assert_eq!(text, json);
    assert!(matches!(
        cql2::parse_file("examples/no-such-file.txt"),
        Err(Error::Io(_))
    ));
}

#[test]
fn the_validator_reports_each_error() {
    let validator = Validator::new().unwrap();
    let valid = json!({"op": "=", "args": [{"property": "a"}, 1]});
    let invalid = json!({"op": "=", "args": [{"property": "a"}]});
    assert!(validator.apply(&valid).is_valid());
    assert!(!validator.apply(&invalid).is_valid());
    assert_eq!(validator.iter_errors(&valid).count(), 0);
    assert!(validator.iter_errors(&invalid).count() > 0);
}

#[test]
fn the_operator_lists_name_the_schema_spellings() {
    assert_eq!(cql2::BOOLOPS, ["and", "or"]);
    assert_eq!(cql2::EQOPS, ["=", "<>"]);
    assert!(cql2::TEMPORALOPS.contains(&"t_metBy"));
    assert!(cql2::ARRAYOPS.contains(&"a_containedBy"));
    for op in cql2::SPATIALOPS {
        let expr: Expr = format!("{op}(g, POINT(0 0))").parse().unwrap();
        assert!(expr.to_text().unwrap().starts_with(op));
    }
}

#[test]
fn spatial_op_evaluates_a_spatial_function() {
    let parse = |text: &str| text.parse::<Expr>().unwrap();
    assert_eq!(
        cql2::spatial_op(parse("POINT(1 1)"), parse("BBOX(0, 0, 2, 2)"), "s_within").unwrap(),
        Expr::Bool(true)
    );
    assert_eq!(
        cql2::spatial_op(parse("POINT(1 1)"), parse("POINT(3 3)"), "ST_Intersects").unwrap(),
        Expr::Bool(false)
    );
    assert!(matches!(
        cql2::spatial_op(parse("POINT(1 1)"), parse("POINT(1 1)"), "t_before"),
        Err(Error::OpNotImplemented(_))
    ));
}

#[test]
fn temporal_op_evaluates_a_temporal_function() {
    let parse = |text: &str| text.parse::<Expr>().unwrap();
    let day = parse("DATE('2020-01-01')");
    let noon = parse("TIMESTAMP('2020-01-01T12:00:00Z')");
    assert_eq!(
        cql2::temporal_op(noon.clone(), day.clone(), "t_during").unwrap(),
        Expr::Bool(true)
    );
    assert_eq!(
        cql2::temporal_op(day.clone(), noon.clone(), "T_CONTAINS").unwrap(),
        Expr::Bool(true)
    );
    assert!(matches!(
        cql2::temporal_op(day, noon, "s_intersects"),
        Err(Error::OpNotImplemented(_))
    ));
}

#[test]
fn a_date_range_runs_from_the_first_instant_to_the_last() {
    use cql2::DateRange;
    let range = |text: &str| DateRange::try_from(text.parse::<Expr>().unwrap()).unwrap();
    let day = range("DATE('2020-01-01')");
    assert_eq!(day.start.to_string(), "2020-01-01T00:00:00Z");
    assert_eq!(day.end.to_string(), "2020-01-01T23:59:59.999999999Z");
    let open = range("INTERVAL('2020-01-01T00:00:00Z', '..')");
    assert_eq!(open.end, jiff::Timestamp::MAX);
    let instant = range("TIMESTAMP('2020-01-02T00:00:00Z')");
    assert_eq!(instant.start, instant.end);
    assert!(day < instant);
    assert_eq!(
        day.partial_cmp(&range("TIMESTAMP('2020-01-01T06:00:00Z')")),
        None
    );
    assert!(matches!(
        DateRange::try_from(Expr::Float(1.0)),
        Err(Error::ExprToDateRange(_))
    ));
}
