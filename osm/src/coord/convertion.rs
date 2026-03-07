use super::{
    geo::{GeoBox, GeoPoint},
    mercator::MercatorPoint,
    screen::ScreenPoint,
};

/// https://wiki.openstreetmap.org/wiki/Mercator#Rust
pub fn mercator_to_geo(pt: &MercatorPoint) -> GeoPoint {
    const EARTH_RADIUS_KM: f64 = 6_378.137;
    GeoPoint::new(
        (2. * ((pt.y() / (EARTH_RADIUS_KM * 1000.)).exp()).atan() - std::f64::consts::PI / 2.)
            .to_degrees(),
        (pt.x() / (EARTH_RADIUS_KM * 1000.)).to_degrees(),
    )
}

/// https://wiki.openstreetmap.org/wiki/Mercator#Rust
pub fn geo_to_mercator(pt: &GeoPoint) -> MercatorPoint {
    const EARTH_RADIUS_KM: f64 = 6_378.137;
    MercatorPoint::new(
        EARTH_RADIUS_KM * 1000. * pt.lon().to_radians(),
        ((pt.lat().to_radians() / 2. + std::f64::consts::PI / 4.).tan()).log(std::f64::consts::E)
            * EARTH_RADIUS_KM
            * 1000.,
    )
}

pub fn geo_to_screen(
    pt: &GeoPoint,
    area: &GeoBox,
    screen_width: f64,
    screen_height: f64,
) -> ScreenPoint {
    ScreenPoint {
        x: screen_width * ((pt.lon() - area.min().lon()) / (area.max().lon() - area.min().lon())),
        y: screen_height
            * (1. - (pt.lat() - area.min().lat()) / (area.max().lat() - area.min().lat())),
    }
}

#[test]
fn mercator_to_geo_test() {
    // New York
    // 40.730610
    // -73.935242

    let new_york = MercatorPoint::new(-8230433.491117454, 4972687.535733603);

    assert_eq!(
        mercator_to_geo(&new_york),
        GeoPoint::new(40.730610, -73.935242),
    );

    assert_eq!(
        mercator_to_geo(&MercatorPoint::new(
            MercatorPoint::LOWER_BOUND,
            MercatorPoint::LOWER_BOUND
        )),
        GeoPoint::new(-85.0511288, -180.0)
    );
    assert_eq!(
        mercator_to_geo(&MercatorPoint::new(
            MercatorPoint::UPPER_BOUND,
            MercatorPoint::UPPER_BOUND
        )),
        GeoPoint::new(85.0511288, 180.0)
    )
}

#[test]
fn geo_to_mercator_test() {
    // New York
    // 40.730610
    // -73.935242

    let new_york = GeoPoint::new(40.730610, -73.935242);

    assert_eq!(
        geo_to_mercator(&new_york),
        MercatorPoint::new(-8230433.491117454, 4972687.535733603)
    );

    // todo!("Find a good way to test approximations")
}

#[test]
fn geo_to_screen_test() {
    assert_eq!(
        geo_to_screen(
            &GeoPoint::new(10., 10.),
            &GeoBox::new(GeoPoint::new(5., 5.), GeoPoint::new(15., 15.)),
            100.,
            100.
        ),
        ScreenPoint { x: 50., y: 50. }
    );

    assert_eq!(
        geo_to_screen(
            &GeoPoint::new(0., 0.),
            &GeoBox::new(GeoPoint::new(-5., -5.), GeoPoint::new(5., 5.)),
            100.,
            100.
        ),
        ScreenPoint { x: 50., y: 50. }
    );

    assert_eq!(
        geo_to_screen(
            // This makes sense since 0,0 is the center, -max, -max is the bottom left, and +max, +max is the top right
            //
            // +-    ++
            //  |    |
            //  |    |
            //  |  x |
            // --    -+
            // Is neg lat(y), pos lon(x)
            &GeoPoint::new(-2.5, 2.5),
            &GeoBox::new(GeoPoint::new(-5., -5.), GeoPoint::new(5., 5.)),
            100.,
            100.
        ),
        ScreenPoint { x: 75., y: 75. }
    );
}
