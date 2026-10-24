use cybermap_server_lib as server_lib;
use osm::coord::geo::*;

#[tokio::test]
async fn chunk_test() {
    logger::init(logger::Config::default().colored(true).filters(&[
        ("warp", log::LevelFilter::Off),
        ("rustls", log::LevelFilter::Off),
        ("h2", log::LevelFilter::Off),
        ("hyper_util", log::LevelFilter::Off),
        ("reqwest", log::LevelFilter::Off),
    ]));

    let bx = serde_json::to_string(&GeoBox::from_center_and_size(
        GeoPoint::new(40.730610, -73.935242),
        (0.15, 0.15), // 0.1, 0.1 panics with what I think to be convertion error
    ))
    .unwrap();
    let fake_base = "https://localhost/";
    let mut url = reqwest::Url::parse(&format!("{fake_base}overpass_chunk_api")).unwrap();
    url.query_pairs_mut().append_pair("inner", &bx);

    let request_path =
        format!("{}?{}", url.path(), url.query().expect("query was added")).replace(fake_base, "");

    let filter = server_lib::chunk_proxy::build_route();

    let response = warp::test::request()
        .path(&request_path)
        .reply(&filter)
        .await;

    println!("Filter response: {response:?}");

    // // Or simply test if a request matches (doesn't reject).
    // assert!(warp::test::request().path("/1/-5").matches(&filter).await);
}

// let bx = serde_json::to_string(&GeoBox::from_center_and_size(
//     GeoPoint::new(40.730610, 73.935242),
//     (1., 1.),
// ))
// .unwrap();
// let mut rb = client.get(format!("http://127.0.0.1:42061/overpass_chunk_api"));
// let rb = rb.query(&[("", &bx)]);
// debug!("{rb:?}");
// // rb.send();
