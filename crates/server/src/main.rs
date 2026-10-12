use std::net::SocketAddr;

fn main() {
    let port: u16 = std::env::args()
        .nth(1)
        .and_then(|arg| arg.parse().ok())
        .unwrap_or(3456);

    let content_root = std::path::PathBuf::from("content");
    let storage_path = std::env::args()
        .position(|arg| arg == "--storage")
        .and_then(|index| std::env::args().nth(index + 1))
        .map(std::path::PathBuf::from);
    let mut app = evr_server::build_server_app(
        content_root,
        evr_server::TickMode::Fixed,
        storage_path.as_deref(),
    )
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
