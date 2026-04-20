use duckdb::{Connection, Result};
use std::{fs, path::PathBuf, sync::OnceLock};

static GEOPARQUET_FIXTURE: OnceLock<PathBuf> = OnceLock::new();

pub fn geoparquet_fixture_path() -> PathBuf {
    GEOPARQUET_FIXTURE
        .get_or_init(|| create_geoparquet_fixture().expect("Failed to prepare GeoParquet fixture"))
        .clone()
}

fn create_geoparquet_fixture() -> Result<PathBuf> {
    let path = std::env::current_dir()
        .expect("Failed to get current directory")
        .join("target/test-data/cql2testdata.parquet");
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("Failed to create fixture directory");
    }
    if path.exists() {
        fs::remove_file(&path).expect("Failed to replace existing fixture file");
    }

    let escaped_path = path
        .to_string_lossy()
        .replace('\\', "/")
        .replace('\'', "''");

    let conn = Connection::open_in_memory()?;
    conn.execute_batch(&format!(
        r#"
        INSTALL SPATIAL;
        LOAD SPATIAL;
        COPY (
            SELECT
                intfield::BIGINT AS intfield,
                st_geomfromgeojson(geom) AS geom,
                textfield::VARCHAR AS textfield,
                floatfield::DOUBLE AS floatfield,
                boolfield::BOOLEAN AS boolfield,
                datefield::TIMESTAMP AS datefield,
                ts_start::TIMESTAMP AS ts_start,
                ts_end::TIMESTAMP AS ts_end,
                intarrayfield::BIGINT[] AS intarrayfield,
                nfield::BOOLEAN AS nfield
            FROM 'tests/cql2testdata.ndjson'
        ) TO '{}' (FORMAT PARQUET);
        "#,
        escaped_path
    ))?;
    Ok(path)
}
