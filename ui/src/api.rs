use std::time::Duration;

use dioxus::prelude::{debug, error};
use osm::{coord::geo::GeoBox, element::NWR};
use reqwest::{Client, StatusCode};

pub enum QueryError {
    Reqwest(reqwest::Error),
    SerdeJson(serde_json::Error),
    API(StatusCode),
}

pub async fn query_with_retries(geobox: GeoBox, url: &str, max_retries: u8) -> Option<NWR> {
    let mut current_try = 0;

    while current_try < max_retries {
        match query(geobox, url).await {
            Ok(data) => return Some(data),
            Err(QueryError::Reqwest(e)) => {
                error!("Overpass api request failled due to: {e}");
                current_try += 1;
                debug!("Retrying");
                continue;
            }
            Err(QueryError::SerdeJson(e)) => {
                error!("Failed to decode Overpass api response due to: {e}");
                return None;
            }
            Err(QueryError::API(status)) => {
                error!("Overpass API returned a bad status code: {status}");
                return None;
            }
        }
    }
    None
}

pub async fn query(geobox: GeoBox, url: &str) -> Result<NWR, QueryError> {
    let query = format!(
        r#"
        [out:json][timeout:360][bbox:{},{},{},{}];
        (
            way
            ['name']
            ['highway']
            ['highway' !~ 'path']
            ['highway' !~ 'steps']
            ['highway' !~ 'motorway']
            ['highway' !~ 'motorway_link']
            ['highway' !~ 'raceway']
            ['highway' !~ 'bridleway']
            ['highway' !~ 'proposed']
            ['highway' !~ 'construction']
            ['highway' !~ 'elevator']
            ['highway' !~ 'bus_guideway']
            ['highway' !~ 'footway']
            ['highway' !~ 'cycleway']
            ['foot' !~ 'no']
            ['access' !~ 'private']
            ['access' !~ 'no'];
        );
        (._;>;);
        out;
        (
            way["building"];
        );
        (._;>;);
        out;
        "#,
        geobox.min().lat(),
        geobox.min().lon(),
        geobox.max().lat(),
        geobox.max().lon()
    );

    let client = Client::new();

    debug!("Query: {query:?}");

    let json_value = {
        let request = client
            .get(url)
            .timeout(Duration::from_secs_f32(10.))
            .header("cybermap", "8b3d00bf-b0cc-4a7d-b389-9c0e9d0688f8")
            .query(&[("data", query)]);

        debug!("Sending: {request:?}");

        let res = request.send().await.map_err(QueryError::Reqwest)?;

        if res.status() != 200 {
            let status = res.status();
            error!(
                "The server responded with a non-200 status code: {}\n{:?}",
                status,
                res.text().await
            );
            return Err(QueryError::API(status));
        }

        let rtext = res.text().await.map_err(QueryError::Reqwest)?;

        match serde_json::from_str(&rtext) {
            Ok(rjson) => rjson,
            Err(e) => {
                error!("Failed to decode rtext into rjson due to: {e}\nThe rtext was: {rtext}");

                panic!();
            }
        }
    };

    osm::parsing::parse_osm_json(json_value).map_err(QueryError::SerdeJson)
}
