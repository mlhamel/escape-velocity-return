use std::net::SocketAddr;

fn main() {
    let port: u16 = std::env::args()
        .nth(1)
        .and_then(|arg| arg.parse().ok())
        .unwrap_or(3456);

    let content_root = std::path::PathBuf::from("content");
    let mut app = evr_server::build_server_app(content_root, evr_server::TickMode::Fixed)
        .expect("content pack must load");
    let bound = evr_server::attach_udp(
        &mut app,
        SocketAddr::from(([0, 0, 0, 0], port)),
        evr_server::MAX_CLIENTS,
    )
    .expect("udp socket must bind");
    println!("evr-server: listening on {bound}");
    app.run();
}
