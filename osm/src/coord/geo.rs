#[derive(Debug, Clone, Copy, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct GeoPoint {
    lat: f64,
    lon: f64,
}

impl GeoPoint {
    pub fn new(mut lat: f64, mut lon: f64) -> Self {
        lat = round(lat, 6);
        lon = round(lon, 6);
        assert!(
            (-90.0..=90.).contains(&lat),
            "Tried to create a GeoPoint with latitude '{lat}' but the allowed range is -90..=90"
        );
        assert!(
            (-180.0..=180.).contains(&lon),
            "Tried to create a GeoPoint with longitude '{lon}' but the allowed range is -180..=180"
        );

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
    pub fn lat(&self) -> f64 {
        self.lat
    }

    pub fn lon(&self) -> f64 {
        self.lon
    }
}

#[derive(Debug, Clone, Copy, PartialEq, serde::Deserialize, serde::Serialize)]
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
        assert!(min.lat < max.lat, "{min:?}, {max:?}");
        assert!(min.lon < max.lon, "{min:?}, {max:?}");

        Self { min, max }
    }

    // https://stackoverflow.com/questions/238260/how-to-calculate-the-bounding-box-for-a-given-lat-lng-location
    pub fn from_center_and_size(center: GeoPoint, size_km: (f64, f64)) -> Self {
        let radius = earth_radius_at_lat(center.lat);

        let lat = center.lat.to_radians();
        let lon = center.lon.to_radians();

        let (halfsize_km_width, halfsize_km_height) = (size_km.0 * 0.5, size_km.1 * 0.5);

        let lat_min = lat - halfsize_km_height / radius;
        let lat_max = lat + halfsize_km_height / radius;

        let pradius = radius * lat.cos();

        let lon_min = lon - halfsize_km_width / pradius;
        let lon_max = lon + halfsize_km_width / pradius;

        Self::new(
            GeoPoint::new(lat_min.to_degrees(), lon_min.to_degrees()),
            GeoPoint::new(lat_max.to_degrees(), lon_max.to_degrees()),
        )
    }
    pub fn to_mercator(&self) -> super::mercator::MercatorBox {
        super::mercator::MercatorBox::new(
            super::convertion::geo_to_mercator(&self.min),
            super::convertion::geo_to_mercator(&self.max),
        )
    }

    /// Rounded at 0.001
    pub fn width_km(&self) -> f64 {
        let center = self.center();

        (center.distance_to(&GeoPoint {
            lat: center.lat,
            lon: self.max.lon,
        }) * 2000.)
            .round()
            / 1000.
    }

    /// Rounded at 0.001
    pub fn height_km(&self) -> f64 {
        let center = self.center();

        (center.distance_to(&GeoPoint {
            lat: self.max.lat,
            lon: center.lon,
        }) * 2000.)
            .round()
            / 1000.
    }

    pub fn center(&self) -> GeoPoint {
        GeoPoint {
            // lat: self.min.lat + self.height() * 0.5,
            // lon: self.min.lon + self.width() * 0.5,
            lat: self.min.lat + (self.max.lat - self.min.lat) * 0.5,
            lon: self.min.lon + (self.max.lon - self.min.lon) * 0.5,
        }
    }
    pub fn min(&self) -> &GeoPoint {
        &self.min
    }
    pub fn max(&self) -> &GeoPoint {
        &self.max
    }
}

#[test]
fn geobox() {
    let coords = (40.730610, -73.935242); // NY
    let size_km = (8.222, 2.111);

    let bx = GeoBox::from_center_and_size(GeoPoint::new(coords.0, coords.1), size_km);

    // println!("Width: {}, expected: {}", bx.width_km(), size_km.0);
    // println!("Height: {}, expected: {}", bx.height_km(), size_km.1);
    assert!(bx.width_km() == size_km.0);
    assert!(bx.height_km() == size_km.1);

    assert_eq!(
        GeoBox::new(
            GeoPoint { lat: 5., lon: 5. },
            GeoPoint { lat: 10., lon: 10. },
        )
        .center(),
        GeoPoint { lat: 7.5, lon: 7.5 }
    )
}

fn round(f: f64, decimals: u32) -> f64 {
    let shift_factor = 10f64.powi(decimals as i32);

    (f * shift_factor).round() / shift_factor
}
