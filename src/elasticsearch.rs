use crate::{Error, Expr, Geometry};
use serde_json::{json, Value};

/// Trait for converting CQL2 expressions to Elasticsearch DSL.
pub trait ToElasticsearch {
    /// Converts this expression to an Elasticsearch DSL query object.
    ///
    /// Returns a [`serde_json::Value`] that represents the Elasticsearch Query DSL.
    ///
    /// # Examples
    ///
    /// ```
    /// use cql2::Expr;
    /// use cql2::ToElasticsearch;
    ///
    /// let expr: Expr = "landsat:scene_id = 'LC82030282019133LGN00'".parse().unwrap();
    /// let dsl = expr.to_elasticsearch().unwrap();
    /// assert_eq!(dsl, serde_json::json!({"term": {"landsat:scene_id": "LC82030282019133LGN00"}}));
    /// ```
    fn to_elasticsearch(&self) -> Result<Value, Error>;
}

/// Converts a CQL2 `LIKE` pattern to an Elasticsearch wildcard pattern.
///
/// CQL2 uses SQL-style wildcards: `%` for any sequence of characters and
/// `_` for any single character. Elasticsearch uses `*` and `?` respectively.
/// Existing `*` and `?` characters in the pattern are escaped with a backslash.
fn like_to_wildcard(pattern: &str) -> String {
    let mut result = String::with_capacity(pattern.len());
    for c in pattern.chars() {
        match c {
            '%' => result.push('*'),
            '_' => result.push('?'),
            '*' | '?' => {
                result.push('\\');
                result.push(c);
            }
            _ => result.push(c),
        }
    }
    result
}

/// Attempts to extract a property name from an expression, handling `casei`/`accenti` wrappers.
///
/// Returns `Some((property_name, case_insensitive))` if the expression is a property reference
/// (optionally wrapped in `casei` or `accenti`), or `None` otherwise.
fn extract_property(expr: &Expr) -> Option<(&str, bool)> {
    match expr {
        Expr::Property { property } => Some((property.as_str(), false)),
        Expr::Operation { op, args } if args.len() == 1 => {
            let op_lower = op.to_lowercase();
            if op_lower == "casei" || op_lower == "accenti" {
                if let Expr::Property { property } = args[0].as_ref() {
                    Some((property.as_str(), op_lower == "casei"))
                } else {
                    None
                }
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Converts a scalar CQL2 expression to a JSON value for use in Elasticsearch queries.
fn scalar_value(expr: &Expr) -> Result<Value, Error> {
    match expr {
        Expr::Float(v) => Ok(json!(*v)),
        Expr::Literal(v) => Ok(json!(v)),
        Expr::Bool(v) => Ok(json!(*v)),
        Expr::Null => Ok(Value::Null),
        Expr::Timestamp { timestamp } => match timestamp.as_ref() {
            Expr::Literal(v) => Ok(json!(v)),
            _ => Ok(json!(timestamp.to_text()?)),
        },
        Expr::Date { date } => match date.as_ref() {
            Expr::Literal(v) => Ok(json!(v)),
            _ => Ok(json!(date.to_text()?)),
        },
        _ => Err(Error::OperationError()),
    }
}

/// Extracts a field name and value from a two-argument comparison expression.
///
/// The property may be on either the left or right side. Returns
/// `(field, case_insensitive, value, flipped)` where `flipped` is `true` when
/// the property was the right-hand argument (so range comparison direction should
/// be reversed).
fn comparison_args(args: &[Box<Expr>]) -> Result<(String, bool, Value, bool), Error> {
    if let Some((field, ci)) = extract_property(args[0].as_ref()) {
        let value = scalar_value(args[1].as_ref())?;
        Ok((field.to_string(), ci, value, false))
    } else if let Some((field, ci)) = extract_property(args[1].as_ref()) {
        let value = scalar_value(args[0].as_ref())?;
        Ok((field.to_string(), ci, value, true))
    } else {
        Err(Error::OperationError())
    }
}

/// Builds an Elasticsearch `geo_shape` query for the given field, geometry, and relation.
fn geo_shape_query(field: &str, geometry: &Geometry, relation: &str) -> Result<Value, Error> {
    // Both GeoJSON and WKT Geometry variants serialize to GeoJSON via the custom serializer.
    let shape = serde_json::to_value(geometry)?;
    Ok(json!({
        "geo_shape": {
            field: {
                "shape": shape,
                "relation": relation
            }
        }
    }))
}

/// Builds an Elasticsearch `geo_shape` query from two spatial-operator arguments.
///
/// Determines which argument is the property (field) and which is the geometry,
/// accepting either order.
fn spatial_args_query(args: &[Box<Expr>], relation: &str) -> Result<Value, Error> {
    let (field, geom_expr) =
        if let Some((prop, _)) = extract_property(args[0].as_ref()) {
            (prop, args[1].as_ref())
        } else if let Some((prop, _)) = extract_property(args[1].as_ref()) {
            (prop, args[0].as_ref())
        } else {
            return Err(Error::OperationError());
        };

    match geom_expr {
        Expr::Geometry(g) => geo_shape_query(field, g, relation),
        Expr::BBox { bbox } => bbox_envelope_query(field, bbox, relation),
        // `BBOX(...)` in CQL2 text form is parsed as Operation { op: "bbox", args }
        Expr::Operation { op, args } if op.to_lowercase() == "bbox" => {
            bbox_envelope_query(field, args, relation)
        }
        _ => Err(Error::OperationError()),
    }
}

/// Builds an Elasticsearch `geo_shape` envelope query from bbox coordinates.
fn bbox_envelope_query(field: &str, bbox: &[Box<Expr>], relation: &str) -> Result<Value, Error> {
    let (minx, miny, maxx, maxy) = extract_bbox_coords(bbox)?;
    Ok(json!({
        "geo_shape": {
            field: {
                "shape": {
                    "type": "envelope",
                    "coordinates": [[minx, maxy], [maxx, miny]]
                },
                "relation": relation
            }
        }
    }))
}

/// Extracts 2D bounding box coordinates `(minx, miny, maxx, maxy)` from a CQL2 bbox array.
fn extract_bbox_coords(bbox: &[Box<Expr>]) -> Result<(f64, f64, f64, f64), Error> {
    let get_float = |expr: &Expr| -> Result<f64, Error> {
        match expr {
            Expr::Float(v) => Ok(*v),
            Expr::Literal(v) => v.parse().map_err(Error::from),
            _ => Err(Error::OperationError()),
        }
    };
    match bbox.len() {
        4 => Ok((
            get_float(bbox[0].as_ref())?,
            get_float(bbox[1].as_ref())?,
            get_float(bbox[2].as_ref())?,
            get_float(bbox[3].as_ref())?,
        )),
        6 => Ok((
            get_float(bbox[0].as_ref())?,
            get_float(bbox[1].as_ref())?,
            get_float(bbox[3].as_ref())?,
            get_float(bbox[4].as_ref())?,
        )),
        _ => Err(Error::OperationError()),
    }
}

/// Extracts a temporal string value from a timestamp, date, or literal expression.
fn temporal_value(expr: &Expr) -> Result<Value, Error> {
    match expr {
        Expr::Timestamp { timestamp } => match timestamp.as_ref() {
            Expr::Literal(v) => Ok(json!(v)),
            _ => Ok(json!(timestamp.to_text()?)),
        },
        Expr::Date { date } => match date.as_ref() {
            Expr::Literal(v) => Ok(json!(v)),
            _ => Ok(json!(date.to_text()?)),
        },
        Expr::Literal(v) => Ok(json!(v)),
        _ => Err(Error::OperationError()),
    }
}

/// Extracts the `(start, end)` temporal extent from an expression.
///
/// For interval expressions, returns the start and end values.
/// For point-in-time expressions (timestamp, date, literal), returns the same
/// value for both start and end.
fn temporal_extent(expr: &Expr) -> Result<(Value, Value), Error> {
    match expr {
        Expr::Interval { interval } => {
            let start = temporal_value(interval[0].as_ref())?;
            let end = temporal_value(interval[1].as_ref())?;
            Ok((start, end))
        }
        _ => {
            let v = temporal_value(expr)?;
            Ok((v.clone(), v))
        }
    }
}

impl ToElasticsearch for Expr {
    /// Converts this expression to an Elasticsearch DSL query.
    ///
    /// # Supported operators
    ///
    /// - **Boolean**: `AND`, `OR`, `NOT`
    /// - **Comparison**: `=`, `<>`, `>`, `>=`, `<`, `<=`
    /// - **Null check**: `IS NULL`
    /// - **Pattern match**: `LIKE` (with optional `casei`/`accenti` wrappers)
    /// - **Membership**: `IN`
    /// - **Range**: `BETWEEN`
    /// - **Spatial**: `S_INTERSECTS`, `S_DISJOINT`, `S_WITHIN`, `S_CONTAINS`,
    ///   `S_EQUALS`, `S_TOUCHES`, `S_OVERLAPS`, `S_CROSSES`
    /// - **Temporal**: `T_BEFORE`, `T_AFTER`, `T_MEETS`, `T_METBY`,
    ///   `T_OVERLAPS`, `T_OVERLAPPEDBY`, `T_STARTS`, `T_STARTEDBY`,
    ///   `T_DURING`, `T_CONTAINS`, `T_FINISHES`, `T_FINISHEDBY`,
    ///   `T_EQUALS`, `T_DISJOINT`, `T_INTERSECTS`, `ANYINTERACTS`
    ///
    /// # Examples
    ///
    /// ```
    /// use cql2::Expr;
    /// use cql2::ToElasticsearch;
    ///
    /// let expr: Expr = "eo:cloud_cover < 0.5".parse().unwrap();
    /// let dsl = expr.to_elasticsearch().unwrap();
    /// assert_eq!(dsl, serde_json::json!({"range": {"eo:cloud_cover": {"lt": 0.5}}}));
    /// ```
    fn to_elasticsearch(&self) -> Result<Value, Error> {
        match self {
            Expr::Bool(true) => Ok(json!({"match_all": {}})),
            Expr::Bool(false) => Ok(json!({"match_none": {}})),
            Expr::Operation { op, args } => {
                let op_lower = op.to_lowercase();
                match op_lower.as_str() {
                    "and" => {
                        let must: Vec<Value> = args
                            .iter()
                            .map(|a| a.to_elasticsearch())
                            .collect::<Result<_, _>>()?;
                        Ok(json!({"bool": {"must": must}}))
                    }
                    "or" => {
                        let should: Vec<Value> = args
                            .iter()
                            .map(|a| a.to_elasticsearch())
                            .collect::<Result<_, _>>()?;
                        Ok(json!({"bool": {"should": should}}))
                    }
                    "not" => {
                        let inner = args[0].to_elasticsearch()?;
                        Ok(json!({"bool": {"must_not": [inner]}}))
                    }
                    "isnull" => {
                        let (field, _) = extract_property(args[0].as_ref())
                            .ok_or_else(|| Error::OperationError())?;
                        Ok(json!({"bool": {"must_not": [{"exists": {"field": field}}]}}))
                    }
                    "between" => {
                        let (field, _) = extract_property(args[0].as_ref())
                            .ok_or_else(|| Error::OperationError())?;
                        let low = scalar_value(args[1].as_ref())?;
                        let high = scalar_value(args[2].as_ref())?;
                        Ok(json!({"range": {field: {"gte": low, "lte": high}}}))
                    }
                    "like" => {
                        let case_insensitive = matches!(
                            args[0].as_ref(),
                            Expr::Operation { op, .. } if op.to_lowercase() == "casei"
                        );
                        let (field, _) = extract_property(args[0].as_ref())
                            .ok_or_else(|| Error::OperationError())?;
                        let pattern = match args[1].as_ref() {
                            Expr::Literal(v) => like_to_wildcard(v),
                            _ => return Err(Error::OperationError()),
                        };
                        if case_insensitive {
                            Ok(json!({"wildcard": {field: {"value": pattern, "case_insensitive": true}}}))
                        } else {
                            Ok(json!({"wildcard": {field: {"value": pattern}}}))
                        }
                    }
                    "in" => {
                        let (field, _) = extract_property(args[0].as_ref())
                            .ok_or_else(|| Error::OperationError())?;
                        let values: Vec<Value> = match args[1].as_ref() {
                            Expr::Array(items) => items
                                .iter()
                                .map(|item| scalar_value(item.as_ref()))
                                .collect::<Result<_, _>>()?,
                            _ => return Err(Error::OperationError()),
                        };
                        Ok(json!({"terms": {field: values}}))
                    }
                    "=" | "eq" | "a_equals" => {
                        let (field, ci, value, _) = comparison_args(args)?;
                        if ci {
                            Ok(json!({"term": {field: {"value": value, "case_insensitive": true}}}))
                        } else {
                            Ok(json!({"term": {field: value}}))
                        }
                    }
                    "<>" | "ne" => {
                        let (field, ci, value, _) = comparison_args(args)?;
                        if ci {
                            Ok(json!({"bool": {"must_not": [{"term": {field: {"value": value, "case_insensitive": true}}}]}}))
                        } else {
                            Ok(json!({"bool": {"must_not": [{"term": {field: value}}]}}))
                        }
                    }
                    ">" | "gt" => {
                        let (field, _, value, flipped) = comparison_args(args)?;
                        let range_op = if flipped { "lt" } else { "gt" };
                        Ok(json!({"range": {field: {range_op: value}}}))
                    }
                    ">=" | "ge" | "gte" => {
                        let (field, _, value, flipped) = comparison_args(args)?;
                        let range_op = if flipped { "lte" } else { "gte" };
                        Ok(json!({"range": {field: {range_op: value}}}))
                    }
                    "<" | "lt" => {
                        let (field, _, value, flipped) = comparison_args(args)?;
                        let range_op = if flipped { "gt" } else { "lt" };
                        Ok(json!({"range": {field: {range_op: value}}}))
                    }
                    "<=" | "le" | "lte" => {
                        let (field, _, value, flipped) = comparison_args(args)?;
                        let range_op = if flipped { "gte" } else { "lte" };
                        Ok(json!({"range": {field: {range_op: value}}}))
                    }
                    "s_intersects" | "st_intersects" | "intersects" => {
                        spatial_args_query(args, "intersects")
                    }
                    "s_within" | "st_within" => spatial_args_query(args, "within"),
                    "s_contains" | "st_contains" => spatial_args_query(args, "contains"),
                    "s_disjoint" | "st_disjoint" => {
                        let inner = spatial_args_query(args, "intersects")?;
                        Ok(json!({"bool": {"must_not": [inner]}}))
                    }
                    "s_equals" | "st_equals" => spatial_args_query(args, "within"),
                    "s_touches" | "st_touches" | "s_overlaps" | "st_overlaps"
                    | "s_crosses" | "st_crosses" => spatial_args_query(args, "intersects"),
                    // Temporal operators: convert to range queries on the property field.
                    // When the property is an interval field, the implementation is an approximation.
                    "t_before" => {
                        // t_before(A, B): end(A) < start(B)
                        if let Some((field, _)) = extract_property(args[0].as_ref()) {
                            let (start, _end) = temporal_extent(args[1].as_ref())?;
                            Ok(json!({"range": {field: {"lt": start}}}))
                        } else if let Some((field, _)) = extract_property(args[1].as_ref()) {
                            let (_start, end) = temporal_extent(args[0].as_ref())?;
                            Ok(json!({"range": {field: {"gt": end}}}))
                        } else {
                            Err(Error::OperationError())
                        }
                    }
                    "t_after" => {
                        // t_after(A, B): start(A) > end(B)
                        if let Some((field, _)) = extract_property(args[0].as_ref()) {
                            let (_start, end) = temporal_extent(args[1].as_ref())?;
                            Ok(json!({"range": {field: {"gt": end}}}))
                        } else if let Some((field, _)) = extract_property(args[1].as_ref()) {
                            let (start, _end) = temporal_extent(args[0].as_ref())?;
                            Ok(json!({"range": {field: {"lt": start}}}))
                        } else {
                            Err(Error::OperationError())
                        }
                    }
                    "t_meets" => {
                        // t_meets(A, B): end(A) = start(B); approximate as A < start(B)
                        if let Some((field, _)) = extract_property(args[0].as_ref()) {
                            let (start, _end) = temporal_extent(args[1].as_ref())?;
                            Ok(json!({"range": {field: {"lt": start}}}))
                        } else if let Some((field, _)) = extract_property(args[1].as_ref()) {
                            let (_start, end) = temporal_extent(args[0].as_ref())?;
                            Ok(json!({"range": {field: {"gt": end}}}))
                        } else {
                            Err(Error::OperationError())
                        }
                    }
                    "t_metby" => {
                        // t_metby(A, B): start(A) = end(B); approximate as A > end(B)
                        if let Some((field, _)) = extract_property(args[0].as_ref()) {
                            let (_start, end) = temporal_extent(args[1].as_ref())?;
                            Ok(json!({"range": {field: {"gt": end}}}))
                        } else if let Some((field, _)) = extract_property(args[1].as_ref()) {
                            let (start, _end) = temporal_extent(args[0].as_ref())?;
                            Ok(json!({"range": {field: {"lt": start}}}))
                        } else {
                            Err(Error::OperationError())
                        }
                    }
                    "t_during" => {
                        // t_during(A, B): start(B) < A < end(B)
                        if let Some((field, _)) = extract_property(args[0].as_ref()) {
                            let (start, end) = temporal_extent(args[1].as_ref())?;
                            Ok(json!({"range": {field: {"gt": start, "lt": end}}}))
                        } else if let Some((field, _)) = extract_property(args[1].as_ref()) {
                            let (start, end) = temporal_extent(args[0].as_ref())?;
                            Ok(json!({"range": {field: {"gt": start, "lt": end}}}))
                        } else {
                            Err(Error::OperationError())
                        }
                    }
                    "t_contains" => {
                        // t_contains(A, B): A contains B (B is during A); approximate as range of B within A
                        if let Some((field, _)) = extract_property(args[1].as_ref()) {
                            let (start, end) = temporal_extent(args[0].as_ref())?;
                            Ok(json!({"range": {field: {"gte": start, "lte": end}}}))
                        } else if let Some((field, _)) = extract_property(args[0].as_ref()) {
                            let (start, end) = temporal_extent(args[1].as_ref())?;
                            Ok(json!({"range": {field: {"gte": start, "lte": end}}}))
                        } else {
                            Err(Error::OperationError())
                        }
                    }
                    "t_starts" | "t_startedby" => {
                        // t_starts(A, B): start(A) = start(B) AND end(A) < end(B)
                        // Approximate: A >= start(B) AND A < end(B)
                        if let Some((field, _)) = extract_property(args[0].as_ref()) {
                            let (start, end) = temporal_extent(args[1].as_ref())?;
                            Ok(json!({"range": {field: {"gte": start, "lt": end}}}))
                        } else if let Some((field, _)) = extract_property(args[1].as_ref()) {
                            let (start, end) = temporal_extent(args[0].as_ref())?;
                            Ok(json!({"range": {field: {"gte": start, "lt": end}}}))
                        } else {
                            Err(Error::OperationError())
                        }
                    }
                    "t_finishes" | "t_finishedby" => {
                        // t_finishes(A, B): end(A) = end(B) AND start(A) > start(B)
                        // Approximate: A > start(B) AND A <= end(B)
                        if let Some((field, _)) = extract_property(args[0].as_ref()) {
                            let (start, end) = temporal_extent(args[1].as_ref())?;
                            Ok(json!({"range": {field: {"gt": start, "lte": end}}}))
                        } else if let Some((field, _)) = extract_property(args[1].as_ref()) {
                            let (start, end) = temporal_extent(args[0].as_ref())?;
                            Ok(json!({"range": {field: {"gt": start, "lte": end}}}))
                        } else {
                            Err(Error::OperationError())
                        }
                    }
                    "t_overlaps" | "t_overlappedby" => {
                        // t_overlaps(A, B): start(A) < end(B) AND start(B) < end(A) AND end(A) < end(B)
                        // Approximate as intersection check
                        if let Some((field, _)) = extract_property(args[0].as_ref()) {
                            let (start, end) = temporal_extent(args[1].as_ref())?;
                            Ok(json!({"range": {field: {"gte": start, "lte": end}}}))
                        } else if let Some((field, _)) = extract_property(args[1].as_ref()) {
                            let (start, end) = temporal_extent(args[0].as_ref())?;
                            Ok(json!({"range": {field: {"gte": start, "lte": end}}}))
                        } else {
                            Err(Error::OperationError())
                        }
                    }
                    "t_equals" => {
                        // t_equals(A, B): start(A) = start(B) AND end(A) = end(B)
                        // For a datetime property, approximate as an exact term match
                        if let Some((field, _)) = extract_property(args[0].as_ref()) {
                            let v = temporal_value(args[1].as_ref())?;
                            Ok(json!({"term": {field: v}}))
                        } else if let Some((field, _)) = extract_property(args[1].as_ref()) {
                            let v = temporal_value(args[0].as_ref())?;
                            Ok(json!({"term": {field: v}}))
                        } else {
                            Err(Error::OperationError())
                        }
                    }
                    "t_disjoint" => {
                        // t_disjoint: NOT t_intersects
                        if let Some((field, _)) = extract_property(args[0].as_ref()) {
                            let (start, end) = temporal_extent(args[1].as_ref())?;
                            Ok(json!({"bool": {"must_not": [{"range": {field: {"gte": start, "lte": end}}}]}}))
                        } else if let Some((field, _)) = extract_property(args[1].as_ref()) {
                            let (start, end) = temporal_extent(args[0].as_ref())?;
                            Ok(json!({"bool": {"must_not": [{"range": {field: {"gte": start, "lte": end}}}]}}))
                        } else {
                            Err(Error::OperationError())
                        }
                    }
                    "t_intersects" | "anyinteracts" => {
                        // t_intersects(A, B): start(A) <= end(B) AND end(A) >= start(B)
                        if let Some((field, _)) = extract_property(args[0].as_ref()) {
                            let (start, end) = temporal_extent(args[1].as_ref())?;
                            Ok(json!({"range": {field: {"gte": start, "lte": end}}}))
                        } else if let Some((field, _)) = extract_property(args[1].as_ref()) {
                            let (start, end) = temporal_extent(args[0].as_ref())?;
                            Ok(json!({"range": {field: {"gte": start, "lte": end}}}))
                        } else {
                            Err(Error::OperationError())
                        }
                    }
                    _ => Err(Error::OpNotImplemented("elasticsearch")),
                }
            }
            _ => Err(Error::OpNotImplemented("elasticsearch")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ToElasticsearch;
    use crate::Expr;
    use serde_json::json;

    #[test]
    fn test_eq_string() {
        let expr: Expr = "landsat:scene_id = 'LC82030282019133LGN00'".parse().unwrap();
        let dsl = expr.to_elasticsearch().unwrap();
        assert_eq!(
            dsl,
            json!({"term": {"landsat:scene_id": "LC82030282019133LGN00"}})
        );
    }

    #[test]
    fn test_eq_number() {
        let expr: Expr = "eo:cloud_cover = 0.1".parse().unwrap();
        let dsl = expr.to_elasticsearch().unwrap();
        assert_eq!(dsl, json!({"term": {"eo:cloud_cover": 0.1}}));
    }

    #[test]
    fn test_ne() {
        let expr: Expr = "eo:cloud_cover <> 0.1".parse().unwrap();
        let dsl = expr.to_elasticsearch().unwrap();
        assert_eq!(
            dsl,
            json!({"bool": {"must_not": [{"term": {"eo:cloud_cover": 0.1}}]}})
        );
    }

    #[test]
    fn test_gt() {
        let expr: Expr = "eo:cloud_cover > 0.1".parse().unwrap();
        let dsl = expr.to_elasticsearch().unwrap();
        assert_eq!(dsl, json!({"range": {"eo:cloud_cover": {"gt": 0.1}}}));
    }

    #[test]
    fn test_gte() {
        let expr: Expr = "eo:cloud_cover >= 0.1".parse().unwrap();
        let dsl = expr.to_elasticsearch().unwrap();
        assert_eq!(dsl, json!({"range": {"eo:cloud_cover": {"gte": 0.1}}}));
    }

    #[test]
    fn test_lt() {
        let expr: Expr = "eo:cloud_cover < 0.5".parse().unwrap();
        let dsl = expr.to_elasticsearch().unwrap();
        assert_eq!(dsl, json!({"range": {"eo:cloud_cover": {"lt": 0.5}}}));
    }

    #[test]
    fn test_lte() {
        let expr: Expr = "eo:cloud_cover <= 0.5".parse().unwrap();
        let dsl = expr.to_elasticsearch().unwrap();
        assert_eq!(dsl, json!({"range": {"eo:cloud_cover": {"lte": 0.5}}}));
    }

    #[test]
    fn test_range_flipped() {
        // Value on the left side: `0.1 < eo:cloud_cover` means `eo:cloud_cover > 0.1`
        let expr: Expr = serde_json::from_str::<Expr>(
            r#"{"op":"<","args":[0.1,{"property":"eo:cloud_cover"}]}"#,
        )
        .unwrap();
        let dsl = expr.to_elasticsearch().unwrap();
        assert_eq!(dsl, json!({"range": {"eo:cloud_cover": {"gt": 0.1}}}));
    }

    #[test]
    fn test_and() {
        let expr: Expr = "beamMode = 'ScanSAR' AND swathDirection = 'ascending'"
            .parse()
            .unwrap();
        let dsl = expr.to_elasticsearch().unwrap();
        assert_eq!(
            dsl,
            json!({
                "bool": {
                    "must": [
                        {"term": {"beamMode": "ScanSAR"}},
                        {"term": {"swathDirection": "ascending"}}
                    ]
                }
            })
        );
    }

    #[test]
    fn test_or() {
        let expr: Expr = "eo:cloud_cover = 0.1 OR eo:cloud_cover = 0.2"
            .parse()
            .unwrap();
        let dsl = expr.to_elasticsearch().unwrap();
        assert_eq!(
            dsl,
            json!({
                "bool": {
                    "should": [
                        {"term": {"eo:cloud_cover": 0.1}},
                        {"term": {"eo:cloud_cover": 0.2}}
                    ]
                }
            })
        );
    }

    #[test]
    fn test_not() {
        let expr: Expr = "NOT landsat:scene_id = 'LC82030282019133LGN00'"
            .parse()
            .unwrap();
        let dsl = expr.to_elasticsearch().unwrap();
        assert_eq!(
            dsl,
            json!({
                "bool": {
                    "must_not": [
                        {"term": {"landsat:scene_id": "LC82030282019133LGN00"}}
                    ]
                }
            })
        );
    }

    #[test]
    fn test_is_null() {
        let expr: Expr = "eo:cloud_cover IS NULL".parse().unwrap();
        let dsl = expr.to_elasticsearch().unwrap();
        assert_eq!(
            dsl,
            json!({"bool": {"must_not": [{"exists": {"field": "eo:cloud_cover"}}]}})
        );
    }

    #[test]
    fn test_between() {
        let expr: Expr = "eo:cloud_cover BETWEEN 0 AND 0.5".parse().unwrap();
        let dsl = expr.to_elasticsearch().unwrap();
        assert_eq!(
            dsl,
            json!({"range": {"eo:cloud_cover": {"gte": 0.0, "lte": 0.5}}})
        );
    }

    #[test]
    fn test_like() {
        let expr: Expr = "eo:instrument LIKE 'OLI%'".parse().unwrap();
        let dsl = expr.to_elasticsearch().unwrap();
        assert_eq!(
            dsl,
            json!({"wildcard": {"eo:instrument": {"value": "OLI*"}}})
        );
    }

    #[test]
    fn test_like_underscore() {
        let expr: Expr = "name LIKE 'ab_def'".parse().unwrap();
        let dsl = expr.to_elasticsearch().unwrap();
        assert_eq!(dsl, json!({"wildcard": {"name": {"value": "ab?def"}}}));
    }

    #[test]
    fn test_like_casei() {
        let expr: Expr = "casei(eo:instrument) LIKE 'oli%'".parse().unwrap();
        let dsl = expr.to_elasticsearch().unwrap();
        assert_eq!(
            dsl,
            json!({"wildcard": {"eo:instrument": {"value": "oli*", "case_insensitive": true}}})
        );
    }

    #[test]
    fn test_in() {
        let expr: Expr = "vehicle:fuel IN ('petrol', 'diesel')".parse().unwrap();
        let dsl = expr.to_elasticsearch().unwrap();
        assert_eq!(
            dsl,
            json!({"terms": {"vehicle:fuel": ["petrol", "diesel"]}})
        );
    }

    #[test]
    fn test_bool_true() {
        let expr = Expr::Bool(true);
        assert_eq!(expr.to_elasticsearch().unwrap(), json!({"match_all": {}}));
    }

    #[test]
    fn test_bool_false() {
        let expr = Expr::Bool(false);
        assert_eq!(expr.to_elasticsearch().unwrap(), json!({"match_none": {}}));
    }

    #[test]
    fn test_casei_eq() {
        let expr: Expr = "casei(eo:instrument) = 'oli_tirs'".parse().unwrap();
        let dsl = expr.to_elasticsearch().unwrap();
        assert_eq!(
            dsl,
            json!({"term": {"eo:instrument": {"value": "oli_tirs", "case_insensitive": true}}})
        );
    }

    #[test]
    fn test_s_intersects_geojson() {
        let expr: Expr = serde_json::from_str::<Expr>(
            r#"{
                "op": "s_intersects",
                "args": [
                    {"property": "footprint"},
                    {"type": "Point", "coordinates": [0.0, 0.0]}
                ]
            }"#,
        )
        .unwrap();
        let dsl = expr.to_elasticsearch().unwrap();
        assert_eq!(
            dsl,
            json!({
                "geo_shape": {
                    "footprint": {
                        "shape": {"type": "Point", "coordinates": [0.0, 0.0]},
                        "relation": "intersects"
                    }
                }
            })
        );
    }

    #[test]
    fn test_s_within() {
        let expr: Expr = serde_json::from_str::<Expr>(
            r#"{
                "op": "s_within",
                "args": [
                    {"property": "location"},
                    {"type": "Polygon", "coordinates": [[[0,0],[1,0],[1,1],[0,1],[0,0]]]}
                ]
            }"#,
        )
        .unwrap();
        let dsl = expr.to_elasticsearch().unwrap();
        assert_eq!(
            dsl["geo_shape"]["location"]["relation"],
            json!("within")
        );
    }

    #[test]
    fn test_s_disjoint() {
        let expr: Expr = serde_json::from_str::<Expr>(
            r#"{
                "op": "s_disjoint",
                "args": [
                    {"property": "footprint"},
                    {"type": "Point", "coordinates": [0.0, 0.0]}
                ]
            }"#,
        )
        .unwrap();
        let dsl = expr.to_elasticsearch().unwrap();
        // Disjoint = NOT intersects
        assert!(dsl["bool"]["must_not"].is_array());
        assert_eq!(
            dsl["bool"]["must_not"][0]["geo_shape"]["footprint"]["relation"],
            json!("intersects")
        );
    }

    #[test]
    fn test_s_intersects_bbox() {
        let expr: Expr = "s_intersects(footprint, BBOX(0, 0, 1, 1))".parse().unwrap();
        let dsl = expr.to_elasticsearch().unwrap();
        assert_eq!(
            dsl,
            json!({
                "geo_shape": {
                    "footprint": {
                        "shape": {
                            "type": "envelope",
                            "coordinates": [[0.0, 1.0], [1.0, 0.0]]
                        },
                        "relation": "intersects"
                    }
                }
            })
        );
    }

    #[test]
    fn test_t_before() {
        let expr: Expr =
            "t_before(datetime, TIMESTAMP('2020-01-01T00:00:00Z'))".parse().unwrap();
        let dsl = expr.to_elasticsearch().unwrap();
        assert_eq!(
            dsl,
            json!({"range": {"datetime": {"lt": "2020-01-01T00:00:00Z"}}})
        );
    }

    #[test]
    fn test_t_after() {
        let expr: Expr =
            "t_after(datetime, TIMESTAMP('2020-01-01T00:00:00Z'))".parse().unwrap();
        let dsl = expr.to_elasticsearch().unwrap();
        assert_eq!(
            dsl,
            json!({"range": {"datetime": {"gt": "2020-01-01T00:00:00Z"}}})
        );
    }

    #[test]
    fn test_t_intersects() {
        let expr: Expr =
            "t_intersects(datetime, INTERVAL('2020-01-01T00:00:00Z','2021-01-01T00:00:00Z'))"
                .parse()
                .unwrap();
        let dsl = expr.to_elasticsearch().unwrap();
        assert_eq!(
            dsl,
            json!({
                "range": {
                    "datetime": {
                        "gte": "2020-01-01T00:00:00Z",
                        "lte": "2021-01-01T00:00:00Z"
                    }
                }
            })
        );
    }

    #[test]
    fn test_t_during() {
        let expr: Expr =
            "t_during(datetime, INTERVAL('2020-01-01T00:00:00Z','2021-01-01T00:00:00Z'))"
                .parse()
                .unwrap();
        let dsl = expr.to_elasticsearch().unwrap();
        assert_eq!(
            dsl,
            json!({
                "range": {
                    "datetime": {
                        "gt": "2020-01-01T00:00:00Z",
                        "lt": "2021-01-01T00:00:00Z"
                    }
                }
            })
        );
    }

    #[test]
    fn test_t_disjoint() {
        let expr: Expr =
            "t_disjoint(datetime, INTERVAL('2020-01-01T00:00:00Z','2021-01-01T00:00:00Z'))"
                .parse()
                .unwrap();
        let dsl = expr.to_elasticsearch().unwrap();
        assert_eq!(
            dsl,
            json!({
                "bool": {
                    "must_not": [{
                        "range": {
                            "datetime": {
                                "gte": "2020-01-01T00:00:00Z",
                                "lte": "2021-01-01T00:00:00Z"
                            }
                        }
                    }]
                }
            })
        );
    }

    #[test]
    fn test_t_equals() {
        let expr: Expr =
            "t_equals(datetime, TIMESTAMP('2020-06-15T00:00:00Z'))".parse().unwrap();
        let dsl = expr.to_elasticsearch().unwrap();
        assert_eq!(
            dsl,
            json!({"term": {"datetime": "2020-06-15T00:00:00Z"}})
        );
    }

    #[test]
    fn test_like_to_wildcard() {
        use super::like_to_wildcard;
        assert_eq!(like_to_wildcard("OLI%"), "OLI*");
        assert_eq!(like_to_wildcard("ab_def"), "ab?def");
        assert_eq!(like_to_wildcard("exact"), "exact");
        assert_eq!(like_to_wildcard("has*star"), "has\\*star");
        assert_eq!(like_to_wildcard("has?q"), "has\\?q");
    }
}
