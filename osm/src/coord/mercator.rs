#[derive(Debug, Clone, Copy, PartialEq, serde::Deserialize)]
pub struct MercatorPoint {
    x: f64,
    y: f64,
}

impl MercatorPoint {
    pub const LOWER_BOUND: f64 = -20037508.3427892;
    pub const UPPER_BOUND: f64 = 20037508.3427892;
    pub fn new(mut x: f64, mut y: f64) -> Self {
        // https://epsg.io/map#srs=3857&x=0&y=0&z=1&layer=streets
        // https://gis.stackexchange.com/questions/144471/spherical-mercator-world-bounds
        // https://history.yale.edu/sites/default/files/files/Bill's%20quick%20guide%20to%20map%20projections.pdf
        // https://store.usgs.gov/assets/mod/storefiles/PDF/16573.pdf
        //
        //
        // xmin: -20037508.3427892,
        // ymin: -20037508.3427892,
        // xmax: 20037508.3427892,
        // ymax: 20037508.3427892
        x = round(x, 7);
        y = round(y, 7);

        assert!(
            (Self::LOWER_BOUND..=Self::UPPER_BOUND).contains(&x),
            "Tried to create a MercatorPoint with X '{x}' but the allowed range is {}..={}",
            Self::LOWER_BOUND,
            Self::UPPER_BOUND
        );
        assert!(
            (Self::LOWER_BOUND..=Self::UPPER_BOUND).contains(&y),
            "Tried to create a MercatorPoint with Y '{y}' but the allowed range is {}..={}",
            Self::LOWER_BOUND,
            Self::UPPER_BOUND
        );

        Self { x, y }
    }

    pub fn x(&self) -> f64 {
        self.x
    }
    pub fn y(&self) -> f64 {
        self.y
    }
}

#[derive(Debug, Clone, Copy, PartialEq, serde::Deserialize)]
pub struct MercatorBox {
    // Bot left
    min: MercatorPoint,

    // Top right
    max: MercatorPoint,
}

impl MercatorBox {
    pub fn new(min: MercatorPoint, max: MercatorPoint) -> Self {
        Self { min, max }
    }

    pub fn min(&self) -> &MercatorPoint {
        &self.min
    }
    pub fn max(&self) -> &MercatorPoint {
        &self.max
    }

    pub fn center(&self) -> MercatorPoint {
        MercatorPoint {
            x: (self.max.x - self.min.x) * 0.5 + self.min.x,
            y: (self.max.y - self.min.y) * 0.5 + self.min.y,
        }
    }
}

fn round(f: f64, decimals: u32) -> f64 {
    let shift_factor = 10f64.powi(decimals as i32);

    (f * shift_factor).round() / shift_factor
}
