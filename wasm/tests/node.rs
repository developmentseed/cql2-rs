//! Test suite for Node.js environment.
//!
//! This file configures the test runner for Node.js and includes the common test suite.

#![cfg(target_arch = "wasm32")]

use wasm_bindgen_test::*;

// Configure tests to run in Node.js
wasm_bindgen_test_configure!(run_in_node_experimental);

// Include all tests from the common module
mod common;
