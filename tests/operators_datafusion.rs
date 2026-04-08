use cql2::{Expr, ToDataFusionSQL};
use datafusion::{
    arrow::{
        array::{Array, ArrayRef, Float64Array, Int64Array, StringArray},
        datatypes::DataType,
        record_batch::RecordBatch,
    },
    common::{
        cast::{as_float64_array, as_string_array},
        Result as DataFusionResult,
    },
    execution::SessionStateBuilder,
    logical_expr::{create_udf, ColumnarValue, Volatility},
    prelude::SessionContext,
};
use geodatafusion_geoparquet::file_format::GeoParquetFormatFactory;
use std::{fs, sync::Arc};
use unaccent::unaccent;

mod geoparquet_fixture;

#[tokio::test]
async fn operators_datafusion_filter() -> DataFusionResult<()> {
    let file_format = Arc::new(GeoParquetFormatFactory::default());
    let state = SessionStateBuilder::new_with_default_features()
        .with_file_formats(vec![file_format])
        .build();
    let ctx = SessionContext::new_with_state(state).enable_url_table();
    geodatafusion::register(&ctx);
    register_strip_accents(&ctx);
    register_power(&ctx);
    let geoparquet = geoparquet_fixture::geoparquet_fixture_path();
    let geoparquet = geoparquet
        .to_string_lossy()
        .replace('\\', "/")
        .replace('\'', "''");

    let tests =
        fs::read_to_string("tests/operators_expected.txt").expect("Failed to read operators tests");
    let mut lines = tests.lines();

    while let Some(query) = lines.next() {
        let expected_line = lines
            .next()
            .unwrap_or_else(|| panic!("Missing expected output for query: {}", query));
        let expr: Expr = query
            .parse()
            .unwrap_or_else(|_| panic!("Failed to parse query '{}'", query));
        let where_clause = expr.to_datafusion_sql().expect("to_datafusion_sql failed");

        let sql = format!(
            "SELECT intfield FROM '{}' AS items \
            WHERE {} \
            ORDER BY intfield",
            geoparquet, where_clause
        );

        let batches = ctx.sql(&sql).await?.collect().await?;
        let result_line = collect_intfields(&batches);
        assert_eq!(
            result_line, expected_line,
            "Query '{}' returned '{}', expected '{}'",
            query, result_line, expected_line
        );
    }

    Ok(())
}

fn register_strip_accents(ctx: &SessionContext) {
    let udf = create_udf(
        "strip_accents",
        vec![DataType::Utf8],
        DataType::Utf8,
        Volatility::Immutable,
        Arc::new(|args| {
            let row_count = match &args[0] {
                ColumnarValue::Array(array) => array.len(),
                ColumnarValue::Scalar(_) => 1,
            };
            let values = args[0].clone().into_array(row_count)?;
            let values = as_string_array(&values)?;
            let normalized = values
                .iter()
                .map(|value| value.map(unaccent))
                .collect::<StringArray>();
            Ok(ColumnarValue::Array(Arc::new(normalized) as ArrayRef))
        }),
    );
    ctx.register_udf(udf);
}

fn register_power(ctx: &SessionContext) {
    let udf = create_udf(
        "power",
        vec![DataType::Float64, DataType::Float64],
        DataType::Float64,
        Volatility::Immutable,
        Arc::new(|args| {
            let row_count = args
                .iter()
                .map(|arg| match arg {
                    ColumnarValue::Array(array) => array.len(),
                    ColumnarValue::Scalar(_) => 1,
                })
                .max()
                .unwrap_or(1);
            let left = args[0].clone().into_array(row_count)?;
            let right = args[1].clone().into_array(row_count)?;
            let left = datafusion::arrow::compute::cast(&left, &DataType::Float64)?;
            let right = datafusion::arrow::compute::cast(&right, &DataType::Float64)?;
            let left = as_float64_array(&left)?;
            let right = as_float64_array(&right)?;
            let values = (0..row_count)
                .map(|i| {
                    if left.is_null(i) || right.is_null(i) {
                        None
                    } else {
                        Some(left.value(i).powf(right.value(i)))
                    }
                })
                .collect::<Float64Array>();
            Ok(ColumnarValue::Array(Arc::new(values) as ArrayRef))
        }),
    );
    ctx.register_udf(udf);
}

fn collect_intfields(batches: &[RecordBatch]) -> String {
    let mut values = Vec::new();
    for batch in batches {
        let array = batch
            .column(0)
            .as_any()
            .downcast_ref::<Int64Array>()
            .expect("intfield column should be Int64");
        values.extend(array.iter().flatten().map(|value| value.to_string()));
    }
    values.join(" ")
}
