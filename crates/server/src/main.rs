use std::path::PathBuf;

fn main() {
    let content_root = PathBuf::from("content");
    let mut app = evr_server::build_app(content_root).expect("content pack must load");
    app.run();
}
