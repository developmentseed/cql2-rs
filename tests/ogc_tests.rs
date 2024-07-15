use cql2_rs::parse;

#[test]
fn geom_operatrion() {
    let expr = parse("S_Within(Point(0 0),geom)");
    assert_eq!(expr.as_json(), "{\"op\":\"s_within\",\"args\":[{\"coordinates\":[0.0,0.0],\"type\":\"Point\"},{\"property\":\"geom\"}]}");
}
