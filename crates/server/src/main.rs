fn main() {
    let content_root = std::path::PathBuf::from("content");
    let mut app = evr_server::build_server_app(content_root, evr_server::TickMode::Fixed)
        .expect("content pack must load");
    app.run();
}
