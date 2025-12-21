mod osm_proxy;

const ADDR: std::net::SocketAddr = std::net::SocketAddr::V4(std::net::SocketAddrV4::new(
    std::net::Ipv4Addr::new(127, 0, 0, 1),
    0xa44d,
));


#[tokio::main]
async fn main() {

    let proxy_route = osm_proxy::build_route();

    println!("Listening on http://{}:{}", ADDR.ip(), ADDR.port());

    warp::serve(proxy_route).run(ADDR).await;
}
