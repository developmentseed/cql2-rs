//! Test suite for web browsers.
//!
//! This file configures the test runner for browsers and includes the common test suite.

#![cfg(target_arch = "wasm32")]

use wasm_bindgen_test::*;

// Configure tests to run in a browser
wasm_bindgen_test_configure!(run_in_browser);

// Include all tests from the common module
mod common;
