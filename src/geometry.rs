use crate::Error;
use geos::{Geom as GeosGeom, Geometry as GeosGeometry};
use geojson::Geometry as GeoJsonGeometry;
use std::str::FromStr;
use serde::{Serialize, Deserialize};

const DEFAULT_NDIM: usize = 2;

/// A CQL2 Geometry.
///
/// # Examples
///
/// ```
/// use cql2::Geometry;
/// let g: Geometry = "POINT(0 0)".parse().unwrap();
/// println!("{:?}", g);
/// assert!(false);
/// ```
///
#[derive(Clone, Serialize, Deserialize)]
pub struct Geometry {
    inner: GeosGeometry,

    #[serde(skip)]
    bbox: bool
}

impl FromStr for Geometry {
    type Err = Error;
    fn from_str(v: &str) -> Result<Geometry, Error> {
        let g = if v.starts_with("{") {
            let j = v.parse::<GeoJsonGeometry>().map_err(Error::from);
            match j {
                Ok(j) => GeosGeometry::try_from(j).map_err(Error::from),
                Err(e) => Err(Error::from(e))
            }
        } else {
            GeosGeometry::new_from_wkt(v).map_err(Error::from)
        };
        match g {
            Ok(g) => Ok(Geometry{inner: g, bbox: false}),
            Err(e) => Err(Error::from(e))
        }
    }
}

impl TryFrom<GeoJsonGeometry> for Geometry {
    type Error = Error;
    fn try_from(v: GeoJsonGeometry) -> Result<Geometry, Error> {
        let g = GeosGeometry::try_from(v).map_err(Error::from);

        match g {
            Ok(g) => Ok(Geometry{inner: g, bbox: false}),
            Err(e) => Err(Error::from(e))
        }
    }
}

impl From<Geometry> for GeosGeometry {
    fn from(v: Geometry) -> GeosGeometry {
        v.inner
    }
}

impl Geometry {
    fn to_wkt(&self) -> Result<String, Error>{
        GeosGeometry::to_wkt(&self.inner).map_err(Error::from)
    }
}


/*
impl CQL2Geom {
    pub fn from_wkt(v: String) -> Self {
        Self::from(v)
    }
    pub fn from_geojson(v: GeoJsonGeometry) -> Self {
        Self::from(v)
    }
    pub fn from_geos(v: GeosGeometry) -> Self {
        Self::from(v)
    }
    pub fn try_from_geojson_string(v: String) -> Result<Self, Error>{
        let g = v.parse::<GeoJsonGeometry>().map_err(Error::from);
        match g {
            Ok(g) => Ok(Self::from(g)),
            Err(e) => Err(e)
        }
    }
}

impl From<GeosGeometry> for CQL2Geom {
    fn from(v: GeosGeometry) -> CQL2Geom {
        CQL2Geom{geojson: None, wkt: None, geos: Some(v)}
    }
}

impl From<GeoJsonGeometry> for CQL2Geom {
    fn from(v: GeoJsonGeometry) -> CQL2Geom {
        CQL2Geom{geojson: Some(v), wkt: None, geos: None}
    }
}

impl From<String> for CQL2Geom {
    fn from(v: String) -> CQL2Geom {
        CQL2Geom{geojson: None, wkt: Some(v), geos: None}
    }
}

impl TryFrom<CQL2Geom> for GeosGeometry {
    type Error = Error;
    fn try_from(v: CQL2Geom) -> Result<GeosGeometry, Error>{
        if let Some(g) = v.geos {
            Ok(g)
        } else if let Some(wkt) = v.wkt {
            GeosGeometry::new_from_wkt(&wkt).map_err(Error::from)
        } else if let Some(geojson) = v.geojson {
            GeosGeometry::try_from(geojson).map_err(Error::from)
        } else {
            Err(Error::ExprToGeom())
        }
    }
}

impl TryFrom<CQL2Geom> for GeoJsonGeometry {
    type Error = Error;
    fn try_from(v: CQL2Geom) -> Result<GeoJsonGeometry, Error>{
        if let Some(g) = v.geojson {
            Ok(g)
        } else if let Some(wkt) = v.wkt {
            let geometry = Wkt(wkt).to_geo().map_err(Error::from);
            match g {
                Ok(g) => geometry.
    geojson::ser::serialize_geometry(&geometry, serializer)
        } else if let Some(geojson) = v.geojson {
            GeosGeometry::try_from(geojson).map_err(Error::from)
        } else {
            Err(Error::ExprToGeom())
        }
    }
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

fn wkt_to_geojson<S>(wkt: &String, serializer: S) -> Result<S::Ok, S::Error>
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
pub fn spatial_op(left: &GeosGeometry, right: &GeosGeometry, op: &str) -> Result<bool, Error> {
    match op {
        "s_equals" => Ok(left == right),
        "s_intersects" | "intersects" => left.intersects(right).map_err(Error::from),
        "s_disjoint" => left.disjoint(right).map_err(Error::from),
        "s_touches" => left.touches(right).map_err(Error::from),
        "s_within" => left.within(right).map_err(Error::from),
        "s_overlaps" => left.overlaps(right).map_err(Error::from),
        "s_crosses" => left.crosses(right).map_err(Error::from),
        "s_contains" => left.contains(right).map_err(Error::from),
        _ => Err(Error::OpNotImplemented("Spatial")),
    }
}
 */
