//! Test suite for the Web and headless browsers.

#![cfg(target_arch = "wasm32")]

extern crate wasm_bindgen_test;
use cql2_wasm::CQL2Expression;
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
fn is_valid() {
    let expr =
        CQL2Expression::new("landsat:scene_id = 'LC82030282019133LGN00'".to_string()).unwrap();
    assert!(expr.is_valid());
}
