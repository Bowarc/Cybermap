#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GeoPoint {
    lat: f64,
    lon: f64,
}

impl GeoPoint {
    pub fn new(lat: f64, lon: f64) -> Self {
        assert!((-90.0..=90.).contains(&lat));
        assert!((-180.0..=180.).contains(&lon));

        Self { lat, lon }
    }

    // haversine formula
    pub fn distance_to(&self, other: &Self) -> f64 {
        // const RADIUS_KM_AVG: f64 = 6371.;
        // For speed I should use the average, but this is more precise
        let radius = (earth_radius_at_lat(self.lat) + earth_radius_at_lat(other.lat)) * 0.5;

        let sin_dlat_half = ((self.lat - other.lat).to_radians() * 0.5).sin();
        let sin_dlon_half = ((self.lon - other.lon).to_radians() * 0.5).sin();

        let a = sin_dlat_half * sin_dlat_half
            + self.lat.to_radians().cos()
                * other.lat.to_radians().cos()
                * sin_dlon_half
                * sin_dlon_half;

        // let c = 2. * a.sqrt().atan2((1. - a).sqrt());
        let c = 2.0 * a.sqrt().asin();
        radius * c
    }
    pub fn lat(&self) -> f64{
        self.lat
    }

    pub fn lon(&self) -> f64{
        self.lon
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GeoBox {
    // Bot left
    min: GeoPoint,

    // Top right
    max: GeoPoint,
}

/// Following the WGS84
/// Takes latitude in degrees
/// Returns value in Km
pub fn earth_radius_at_lat(lat: f64) -> f64 {
    const WGS84_A: f64 = 6_378.137;
    const WGS84_A2: f64 = WGS84_A * WGS84_A;
    const WGS84_B: f64 = 6_356.752; 
    const WGS84_B2: f64 = WGS84_B * WGS84_B;

    let lat = lat.to_radians();

    let latcos = lat.cos();
    let latsin = lat.sin();

    let an = WGS84_A2 * latcos;
    let bn = WGS84_B2 * latsin;
    let ad = WGS84_A * latcos;
    let bd = WGS84_B * latsin;

    ((an * an + bn * bn) / (ad * ad + bd * bd)).sqrt()
}

impl GeoBox {
    pub fn new(min: GeoPoint, max: GeoPoint) -> Self {
        assert!(min.lat < max.lat);
        assert!(min.lon < max.lon);

        Self { min, max }
    }

    // https://stackoverflow.com/questions/238260/how-to-calculate-the-bounding-box-for-a-given-lat-lng-location
    pub fn from_center_and_size(center: GeoPoint, size_km: f64) -> Self {
        let radius = earth_radius_at_lat(center.lat);

        let lat = center.lat.to_radians();
        let lon = center.lon.to_radians();

        let lat_min = lat - size_km / radius;
        let lat_max = lat + size_km / radius;

        let pradius = radius * lat.cos();

        let lon_min = lon - size_km / pradius;
        let lon_max = lon + size_km / pradius;

        Self::new(
            GeoPoint::new(lat_min.to_degrees(), lon_min.to_degrees()),
            GeoPoint::new(lat_max.to_degrees(), lon_max.to_degrees()),
        )
    }
    pub fn width(&self) -> f64 {
        self.max.lon - self.min.lon
    }
    pub fn height(&self) -> f64 {
        self.max.lat - self.min.lat
    }
    pub fn center(&self) -> GeoPoint {
        GeoPoint {
            lat: self.min.lat + self.height() * 0.5,
            lon: self.min.lon + self.width() * 0.5,
        }
    }
    pub fn min(&self) -> &GeoPoint{
        &self.min
    }
    pub fn max(&self) -> &GeoPoint{
        &self.max
    }
}
