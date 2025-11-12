//! Shared WASM test suite for both Node.js and browser environments.
//!
//! This module contains tests that run in both environments. The test runner
//! is configured based on which test file includes this module.

#![cfg(target_arch = "wasm32")]

use cql2_wasm::{parse_json, parse_text, CQL2Expression};
use wasm_bindgen_test::*;

#[wasm_bindgen_test]
fn is_valid() {
    let expr =
        CQL2Expression::new("landsat:scene_id = 'LC82030282019133LGN00'".to_string()).unwrap();
    assert!(expr.is_valid());
}

#[wasm_bindgen_test]
fn test_parse_text() {
    let expr = parse_text("landsat:scene_id = 'LC82030282019133LGN00'").unwrap();
    assert!(expr.is_valid());
}

#[wasm_bindgen_test]
fn test_parse_json() {
    let json = r#"{"op":"=","args":[{"property":"landsat:scene_id"},"LC82030282019133LGN00"]}"#;
    let expr = parse_json(json).unwrap();
    assert!(expr.is_valid());
}

#[wasm_bindgen_test]
fn test_validate_valid_expression() {
    let expr =
        CQL2Expression::new("landsat:scene_id = 'LC82030282019133LGN00'".to_string()).unwrap();
    assert!(expr.validate().is_ok());
}

#[wasm_bindgen_test]
fn test_to_json() {
    let expr =
        CQL2Expression::new("landsat:scene_id = 'LC82030282019133LGN00'".to_string()).unwrap();
    let json = expr.to_json().unwrap();
    assert!(json.contains("landsat:scene_id"));
    assert!(json.contains("LC82030282019133LGN00"));
}

#[wasm_bindgen_test]
fn test_to_json_pretty() {
    let expr =
        CQL2Expression::new("landsat:scene_id = 'LC82030282019133LGN00'".to_string()).unwrap();
    let json = expr.to_json_pretty().unwrap();
    assert!(json.contains("landsat:scene_id"));
    assert!(json.contains("\n")); // Should have newlines for pretty printing
}

#[wasm_bindgen_test]
fn test_to_text() {
    let json = r#"{"op":"=","args":[{"property":"landsat:scene_id"},"LC82030282019133LGN00"]}"#;
    let expr = parse_json(json).unwrap();
    let text = expr.to_text().unwrap();
    assert!(text.contains("landsat:scene_id"));
    assert!(text.contains("LC82030282019133LGN00"));
}

#[wasm_bindgen_test]
fn test_to_sql() {
    let expr =
        CQL2Expression::new("landsat:scene_id = 'LC82030282019133LGN00'".to_string()).unwrap();
    let sql = expr.to_sql().unwrap();
    assert!(sql.contains("landsat:scene_id"));
    assert!(sql.contains("LC82030282019133LGN00"));
}

#[wasm_bindgen_test]
fn test_matches_with_matching_item() {
    let expr = CQL2Expression::new("id = 1".to_string()).unwrap();
    let item = r#"{"id": 1, "name": "test"}"#;
    let result = expr.matches(Some(item.to_string())).unwrap();
    assert!(result);
}

#[wasm_bindgen_test]
fn test_matches_with_non_matching_item() {
    let expr = CQL2Expression::new("id = 1".to_string()).unwrap();
    let item = r#"{"id": 2, "name": "test"}"#;
    let result = expr.matches(Some(item.to_string())).unwrap();
    assert!(!result);
}

#[wasm_bindgen_test]
fn test_matches_without_item() {
    let expr = CQL2Expression::new("true".to_string()).unwrap();
    let result = expr.matches(None).unwrap();
    assert!(result);
}

#[wasm_bindgen_test]
fn test_reduce_without_item() {
    let expr = CQL2Expression::new("1 + 2".to_string()).unwrap();
    let reduced = expr.reduce(None).unwrap();
    let text = reduced.to_text().unwrap();
    assert_eq!(text, "3");
}

#[wasm_bindgen_test]
fn test_reduce_with_item() {
    let expr = CQL2Expression::new("id + 10".to_string()).unwrap();
    let item = r#"{"id": 5}"#;
    let reduced = expr.reduce(Some(item.to_string())).unwrap();
    let text = reduced.to_text().unwrap();
    assert_eq!(text, "15");
}

#[wasm_bindgen_test]
fn test_add_expressions() {
    let expr1 = CQL2Expression::new("id = 1".to_string()).unwrap();
    let expr2 = CQL2Expression::new("name = 'test'".to_string()).unwrap();
    let combined = expr1.add(&expr2);
    let text = combined.to_text().unwrap();
    assert!(text.contains("id"));
    assert!(text.contains("name"));
    assert!(text.contains("AND") || text.contains("and"));
}

#[wasm_bindgen_test]
fn test_equals_same_expressions() {
    let expr1 = CQL2Expression::new("id = 1".to_string()).unwrap();
    let expr2 = CQL2Expression::new("id = 1".to_string()).unwrap();
    assert!(expr1.equals(&expr2));
}

#[wasm_bindgen_test]
fn test_equals_different_expressions() {
    let expr1 = CQL2Expression::new("id = 1".to_string()).unwrap();
    let expr2 = CQL2Expression::new("id = 2".to_string()).unwrap();
    assert!(!expr1.equals(&expr2));
}
