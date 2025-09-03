use std::cmp::Ordering;

use crate::{Error, Expr};
use geo::*;
use geo_types::Geometry as GGeom;
use geozero::{wkt::Wkt, CoordDimensions, ToGeo, ToWkt};
use serde::{Deserialize, Serialize, Serializer};

const DEFAULT_NDIM: usize = 2;

/// Crate-specific geometry type to hold either WKT or GeoJSON.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum Geometry {
    /// A GeoJSON geometry.
    GeoJSON(geojson::Geometry),

    /// A WKT geometry.
    #[serde(skip_deserializing, serialize_with = "to_geojson")]
    Wkt(String),
}

impl Geometry {
    /// Converts this geometry to Well-Known Text (WKT).
    ///
    /// # Examples
    ///
    /// ```
    /// use cql2::Geometry;
    ///
    /// let geometry: Geometry = serde_json::from_str(
    ///      "{\"type\":\"Point\",\"coordinates\":[-105.1019,40.1672]}"
    /// ).unwrap();
    /// assert_eq!("POINT(-105.1019 40.1672)", geometry.to_wkt().unwrap());
    /// ```
    pub fn to_wkt(&self) -> Result<String, Error> {
        match self {
            Geometry::Wkt(wkt) => Ok(wkt.clone()),
            Geometry::GeoJSON(geojson) => {
                let dims = match geojson_ndims(geojson) {
                    3 => CoordDimensions::xyz(),
                    4 => CoordDimensions::xyzm(),
                    _ => CoordDimensions::xy(),
                };
                let geometry: geo_types::Geometry<f64> = geojson.clone().try_into()?;
                geometry.to_wkt_ndim(dims).map_err(Error::from)
            }
        }
    }
}

impl PartialEq for Geometry {
    fn eq(&self, other: &Self) -> bool {
        let left = Expr::Geometry(self.clone());
        let right = Expr::Geometry(other.clone());
        let v = spatial_op(left, right, "s_equals").unwrap_or(Expr::Bool(false));
        match v {
            Expr::Bool(v) => v,
            _ => false,
        }
    }
}

impl PartialOrd for Geometry {
    fn partial_cmp(&self, _other: &Self) -> Option<Ordering> {
        None
    }
}

fn to_geojson<S>(wkt: &String, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    use serde::ser::Error;

    let geometry = Wkt(wkt).to_geo().map_err(Error::custom)?;
    geojson::ser::serialize_geometry(&geometry, serializer)
}

fn geojson_ndims(geojson: &geojson::Geometry) -> usize {
    use geojson::Value::*;
    match &geojson.value {
        Point(coords) => coords.len(),
        MultiPoint(v) => v.first().map(|v| v.len()).unwrap_or(DEFAULT_NDIM),
        LineString(v) => v.first().map(|v| v.len()).unwrap_or(DEFAULT_NDIM),
        MultiLineString(v) => v
            .first()
            .and_then(|v| v.first())
            .map(|v| v.len())
            .unwrap_or(DEFAULT_NDIM),
        Polygon(v) => v
            .first()
            .and_then(|v| v.first())
            .map(|v| v.len())
            .unwrap_or(DEFAULT_NDIM),
        MultiPolygon(v) => v
            .first()
            .and_then(|v| v.first())
            .and_then(|v| v.first())
            .map(|v| v.len())
            .unwrap_or(DEFAULT_NDIM),
        GeometryCollection(v) => v.first().map(geojson_ndims).unwrap_or(DEFAULT_NDIM),
    }
}

/// Run a spatial operation.
pub fn spatial_op(left: Expr, right: Expr, op: &str) -> Result<Expr, Error> {
    let left: GGeom = GGeom::try_from(left)?;
    let right: GGeom = GGeom::try_from(right)?;
    let rel = left.relate(&right);
    let out = match op {
        "s_equals" => rel.is_equal_topo(),
        "s_intersects" | "intersects" => rel.is_intersects(),
        "s_disjoint" => rel.is_disjoint(),
        "s_touches" => rel.is_touches(),
        "s_within" => rel.is_within(),
        "s_overlaps" => rel.is_overlaps(),
        "s_crosses" => rel.is_crosses(),
        "s_contains" => rel.is_contains(),
        &_ => todo!(),
    };
    Ok(Expr::Bool(out))
}
