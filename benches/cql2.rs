//! Benchmarks on the crate's own fixtures. Run with `cargo bench --bench cql2`.

use cql2::{Expr, ToSqlAst};
use criterion::{criterion_group, criterion_main, BatchSize, Criterion};
use serde_json::Value;
use std::{fs, hint::black_box};

/// The contents of every `.extension` file in `dir`, in name order.
fn read_dir(dir: &str, extension: &str) -> Vec<String> {
    let mut paths: Vec<_> = fs::read_dir(dir)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.extension().is_some_and(|e| e == extension))
        .collect();
    paths.sort();
    paths
        .iter()
        .map(|path| fs::read_to_string(path).unwrap())
        .collect()
}

fn read_ndjson(path: &str) -> Vec<Value> {
    fs::read_to_string(path)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}

/// The queries of `tests/operators_tests.txt`.
fn operator_queries() -> Vec<String> {
    fs::read_to_string("tests/operators_tests.txt")
        .unwrap()
        .lines()
        .filter(|line| !line.is_empty() && !line.starts_with("//"))
        .map(String::from)
        .collect()
}

fn parse_all(texts: &[String]) -> Vec<Expr> {
    texts.iter().map(|text| text.parse().unwrap()).collect()
}

/// A STAC API search filter.
const STAC: &str = "collection = 'landsat-c2-l2' AND eo:cloud_cover < 20 AND eo:cloud_cover < 50 \
    AND datetime >= TIMESTAMP('2021-01-01T00:00:00Z') AND datetime < TIMESTAMP('2022-01-01T00:00:00Z') \
    AND s_intersects(geometry, POLYGON((-105 39, -104 39, -104 40, -105 40, -105 39))) \
    AND platform IN ('landsat-8', 'landsat-9') AND NOT (instruments = 'tirs')";

/// Long filters over the properties of `tests/cql2testdata.ndjson`.
fn large_filters() -> Vec<(&'static str, Expr)> {
    let join = |terms: Vec<String>, op: &str| terms.join(&format!(" {op} "));
    let or_of_ands = join(
        (0..200)
            .map(|i| format!("(intfield = {i} AND textfield = 'item_{i}' AND floatfield > {i}.5)"))
            .collect(),
        "OR",
    );
    let and_of_ranges = join(
        (0..300)
            .map(|i| match i % 3 {
                0 => format!("intfield > {i}"),
                1 => format!("floatfield < {}", 1000 - i),
                _ => format!("datefield >= DATE('2020-{:02}-01')", i % 12 + 1),
            })
            .collect(),
        "AND",
    );
    let list =
        |range: std::ops::Range<i32>| range.map(|i| i.to_string()).collect::<Vec<_>>().join(", ");
    let in_lists = format!(
        "intfield IN ({}) AND intfield IN ({}) AND textfield IN ({})",
        list(0..1000),
        list(500..1500),
        (0..500)
            .map(|i| format!("'item_{i}'"))
            .collect::<Vec<_>>()
            .join(", ")
    );
    vec![
        ("or_of_ands", or_of_ands.parse().unwrap()),
        ("and_of_ranges", and_of_ranges.parse().unwrap()),
        ("in_lists", in_lists.parse().unwrap()),
    ]
}

fn parse(c: &mut Criterion) {
    let text = read_dir("examples/text", "txt");
    let operators = operator_queries();
    let json = read_dir("examples/json", "json");
    let mut group = c.benchmark_group("parse");
    group.bench_function("text_examples", |b| b.iter(|| parse_all(black_box(&text))));
    group.bench_function("text_operators", |b| {
        b.iter(|| parse_all(black_box(&operators)))
    });
    group.bench_function("json_examples", |b| b.iter(|| parse_all(black_box(&json))));
    group.finish();
}

fn reduce(c: &mut Criterion) {
    let examples = parse_all(&read_dir("examples/text", "txt"));
    let stac: Expr = STAC.parse().unwrap();
    let mut group = c.benchmark_group("reduce");
    let mut bench = |name: &str, exprs: Vec<Expr>| {
        group.bench_function(name, |b| {
            b.iter_batched(
                || exprs.clone(),
                |exprs| {
                    for expr in exprs {
                        black_box(expr.reduce(None).unwrap());
                    }
                },
                BatchSize::SmallInput,
            )
        });
    };
    bench("examples", examples);
    bench("stac", vec![stac]);
    for (name, expr) in large_filters() {
        bench(name, vec![expr]);
    }
    group.finish();
}

fn filter(c: &mut Criterion) {
    let items = read_ndjson("tests/cql2testdata.ndjson");
    let operators = parse_all(&operator_queries());
    let mut group = c.benchmark_group("filter");
    group.bench_function("operators", |b| {
        b.iter(|| {
            for expr in &operators {
                black_box(expr.filter(&items).unwrap());
            }
        })
    });
    group.finish();
}

fn output(c: &mut Criterion) {
    let mut inputs = vec![("examples", parse_all(&read_dir("examples/text", "txt")))];
    inputs.extend(
        large_filters()
            .into_iter()
            .map(|(name, expr)| (name, vec![expr])),
    );
    bench_each(c, "to_text", &inputs, |expr| expr.to_text().unwrap());
    bench_each(c, "to_sql", &inputs, |expr| expr.to_sql().unwrap());
}

/// Times `run` on each named list of expressions.
fn bench_each(
    c: &mut Criterion,
    group: &str,
    inputs: &[(&str, Vec<Expr>)],
    run: fn(&Expr) -> String,
) {
    let mut group = c.benchmark_group(group);
    for (name, exprs) in inputs {
        group.bench_function(*name, |b| {
            b.iter(|| {
                for expr in exprs {
                    black_box(run(black_box(expr)));
                }
            })
        });
    }
    group.finish();
}

criterion_group!(benches, parse, reduce, filter, output);
criterion_main!(benches);
