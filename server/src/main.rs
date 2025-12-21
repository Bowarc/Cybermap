#[macro_use]
extern crate log;

mod osm_proxy;

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
            .colored(true),
    );

    trace!(
        "\n╭{line}╮\n│{message:^30}│\n╰{line}╯",
        line = "─".repeat(30),
        message = "Server start"
    );

    let proxy_route = osm_proxy::build_route();

    debug!("Listening on http://{}:{}", ADDR.ip(), ADDR.port());

    warp::serve(proxy_route).run(ADDR).await;
}
