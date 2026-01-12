use warp::Filter;

#[macro_use]
extern crate log;

mod osm_proxy;
mod rejection;

const ADDR: std::net::SocketAddr = std::net::SocketAddr::V4(std::net::SocketAddrV4::new(
    std::net::Ipv4Addr::new(127, 0, 0, 1),
    0xa44d,
));

#[tokio::main]
async fn main() {
    logger::init(
        logger::Config::default()
            .output(logger::Output::Stdout)
            .filter("warp", log::LevelFilter::Debug)
            .filter("hyper_util", log::LevelFilter::Warn)
            .filter("reqwest", log::LevelFilter::Warn)
            .colored(true),
    );

    trace!(
        "\n╭{line}╮\n│{message:^30}│\n╰{line}╯",
        line = "─".repeat(30),
        message = "Server start"
    );

    let proxy_route = osm_proxy::build_route();
    let static_route = warp::get()
        .and(warp::fs::dir("static"))
        .recover(rejection::not_found);

    debug!("Listening on http://{}:{}", ADDR.ip(), ADDR.port());

    warp::serve(
        proxy_route
            .or(static_route)
            .recover(rejection::unknown),
    )
    .run(ADDR)
    .await;
}
