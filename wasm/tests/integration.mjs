/**
 * JavaScript integration tests for CQL2 WASM bindings
 *
 * These tests verify that the WASM module exposes the correct JavaScript API
 * and that it works correctly from JavaScript code.
 *
 * Supports both nodejs and web targets by using async initialization.
 */

import { strict as assert } from 'assert';
import { readFile } from 'fs/promises';
import { fileURLToPath } from 'url';
import { dirname, join } from 'path';
import initModule, { Expr, parseText, parseJson } from "../pkg/cql2_wasm.js";

const __dirname = dirname(fileURLToPath(import.meta.url));

let testCount = 0;
let passCount = 0;
let failCount = 0;

async function test(name, fn) {
  testCount++;
  try {
    await fn();
    passCount++;
    console.log(`✓ ${name}`);
  } catch (error) {
    failCount++;
    console.error(`✗ ${name}`);
    console.error(`  ${error.message}`);
  }
}

console.log('\nRunning JavaScript integration tests...\n');

// Initialize WASM module (required for web target, no-op for nodejs target)
// For nodejs target, the default export is undefined and WASM auto-initializes
// For web target, the default export is the init function that must be called
if (typeof initModule === 'function') {
  // Web target in Node.js - need to load WASM file manually
  const wasmPath = join(__dirname, '..', 'pkg', 'cql2_wasm_bg.wasm');
  const wasmBytes = await readFile(wasmPath);
  await initModule({ module_or_path: wasmBytes });
}

// Run all tests
await (async () => {
  // Test module-level functions
  await test('parseText exports a function', () => {
    assert.equal(typeof parseText, 'function');
  });

  await test('parseJson exports a function', () => {
    assert.equal(typeof parseJson, 'function');
  });

  await test('CQL2 is a constructor', () => {
    assert.equal(typeof Expr, "function");
  });

  // Test parseText function
  await test("parseText() parses CQL2 text format", () => {
    const expr = parseText("landsat:scene_id = 'LC82030282019133LGN00'");
    assert.ok(expr);
    assert.ok(expr instanceof Expr);
  });

  await test("parseText() handles complex syntax", () => {
    // The parser is quite lenient, so we test it handles various inputs
    const expr = parseText("id = 1 AND name = 'test'");
    assert.ok(expr);
  });

  // Test parseJson function
  await test("parseJson() parses CQL2 JSON format", () => {
    const json =
      '{"op":"=","args":[{"property":"landsat:scene_id"},"LC82030282019133LGN00"]}';
    const expr = parseJson(json);
    assert.ok(expr);
    assert.ok(expr instanceof Expr);
  });

  await test('parseJson() throws on invalid JSON', () => {
    assert.throws(() => {
      parseJson("not valid json");
    });
  });

  // Test CQL2 constructor
  await test("new CQL2() creates expression from text", () => {
    const expr = new Expr("id = 1");
    assert.ok(expr);
    assert.ok(expr instanceof Expr);
  });

  await test("new CQL2() handles various operators", () => {
    const expr = new Expr("value > 100");
    assert.ok(expr);
    assert.ok(expr instanceof Expr);
  });

  // Test validate() method
  await test("validate() method exists and works", () => {
    const expr = new Expr("id = 1");
    assert.equal(typeof expr.validate, "function");
    expr.validate(); // Should not throw
  });

  await test("validate() validates against JSON schema", () => {
    const expr = new Expr("landsat:scene_id = 'LC82030282019133LGN00'");
    expr.validate(); // Should not throw
  });

  // Test is_valid() method
  await test("is_valid() method exists and returns boolean", () => {
    const expr = new Expr("id = 1");
    assert.equal(typeof expr.is_valid, "function");
    assert.equal(typeof expr.is_valid(), "boolean");
  });

  await test("is_valid() returns true for valid expression", () => {
    const expr = new Expr("id = 1");
    assert.equal(expr.is_valid(), true);
  });

  // Test matches() method
  await test("matches() method exists", () => {
    const expr = new Expr("id = 1");
    assert.equal(typeof expr.matches, "function");
  });

  await test("matches() returns true for matching item", () => {
    const expr = new Expr("id = 1");
    const item = JSON.stringify({ id: 1, name: "test" });
    const result = expr.matches(item);
    assert.equal(result, true);
  });

  await test("matches() returns false for non-matching item", () => {
    const expr = new Expr("id = 1");
    const item = JSON.stringify({ id: 2, name: "test" });
    const result = expr.matches(item);
    assert.equal(result, false);
  });

  await test("matches() works without item (null/undefined)", () => {
    const expr = new Expr("true");
    const result = expr.matches(null);
    assert.equal(result, true);
  });

  // Test reduce() method
  await test("reduce() method exists", () => {
    const expr = new Expr("1 + 2");
    assert.equal(typeof expr.reduce, "function");
  });

  await test("reduce() simplifies expressions without item", () => {
    const expr = new Expr("1 + 2");
    const reduced = expr.reduce(null);
    assert.ok(reduced instanceof Expr);
    const text = reduced.to_text();
    assert.equal(text, "3");
  });

  await test("reduce() simplifies expressions with item context", () => {
    const expr = new Expr("id + 10");
    const item = JSON.stringify({ id: 5 });
    const reduced = expr.reduce(item);
    const text = reduced.to_text();
    assert.equal(text, "15");
  });

  // Test to_json() method
  await test("to_json() method exists and returns string", () => {
    const expr = new Expr("id = 1");
    assert.equal(typeof expr.to_json, "function");
    const json = expr.to_json();
    assert.equal(typeof json, "string");
  });

  await test("to_json() returns valid JSON", () => {
    const expr = new Expr("landsat:scene_id = 'LC82030282019133LGN00'");
    const json = expr.to_json();
    const parsed = JSON.parse(json); // Should not throw
    assert.ok(parsed.args);
  });

  // Test to_json_pretty() method
  await test("to_json_pretty() method exists and returns formatted JSON", () => {
    const expr = new Expr("id = 1");
    assert.equal(typeof expr.to_json_pretty, "function");
    const json = expr.to_json_pretty();
    assert.equal(typeof json, "string");
    assert.ok(json.includes("\n")); // Should have newlines
  });

  // Test to_text() method
  await test("to_text() method exists and returns string", () => {
    const expr = new Expr("id = 1");
    assert.equal(typeof expr.to_text, "function");
    const text = expr.to_text();
    assert.equal(typeof text, "string");
  });

  await test('to_text() converts JSON to text format', () => {
    const json = '{"op":"=","args":[{"property":"landsat:scene_id"},"LC82030282019133LGN00"]}';
    const expr = parseJson(json);
    const text = expr.to_text();
    assert.ok(text.includes("landsat:scene_id"));
    assert.ok(text.includes("LC82030282019133LGN00"));
  });

  // Test to_sql() method
  await test("to_sql() method exists and returns string", () => {
    const expr = new Expr("id = 1");
    assert.equal(typeof expr.to_sql, "function");
    const sql = expr.to_sql();
    assert.equal(typeof sql, "string");
  });

  await test("to_sql() converts expression to SQL", () => {
    const expr = new Expr("landsat:scene_id = 'LC82030282019133LGN00'");
    const sql = expr.to_sql();
    assert.ok(
      sql.includes("landsat:scene_id") || sql.includes("landsat_scene_id")
    );
    assert.ok(sql.includes("LC82030282019133LGN00"));
  });

  // Test add() method
  await test("add() method exists", () => {
    const expr1 = new Expr("id = 1");
    assert.equal(typeof expr1.add, "function");
  });

  await test("add() combines two expressions with AND", () => {
    const expr1 = new Expr("id = 1");
    const expr2 = new Expr("name = 'test'");
    const combined = expr1.add(expr2);
    assert.ok(combined instanceof Expr);
    const text = combined.to_text();
    assert.ok(text.includes("id"));
    assert.ok(text.includes("name"));
    assert.ok(text.toLowerCase().includes("and"));
  });

  // Test equals() method
  await test("equals() method exists and returns boolean", () => {
    const expr1 = new Expr("id = 1");
    const expr2 = new Expr("id = 1");
    assert.equal(typeof expr1.equals, "function");
    assert.equal(typeof expr1.equals(expr2), "boolean");
  });

  await test("equals() returns true for identical expressions", () => {
    const expr1 = new Expr("id = 1");
    const expr2 = new Expr("id = 1");
    assert.equal(expr1.equals(expr2), true);
  });

  await test("equals() returns false for different expressions", () => {
    const expr1 = new Expr("id = 1");
    const expr2 = new Expr("id = 2");
    assert.equal(expr1.equals(expr2), false);
  });

  // Test round-trip conversions
  await test('Round-trip: text -> JSON -> text preserves meaning', () => {
    const original = "id = 42 AND name = 'test'";
    const expr = parseText(original);
    const json = expr.to_json();
    const expr2 = parseJson(json);
    const final = expr2.to_text();
    // Both should be semantically equivalent (though formatting may differ)
    assert.ok(expr.equals(expr2));
  });
})();

// Print summary
console.log('\n' + '='.repeat(50));
console.log(`Tests: ${testCount}`);
console.log(`Passed: ${passCount}`);
console.log(`Failed: ${failCount}`);
console.log('='.repeat(50));

if (failCount > 0) {
  process.exit(1);
}
