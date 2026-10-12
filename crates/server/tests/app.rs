use std::path::Path;

use evr_server::Content;

#[test]
fn server_app_loads_base_pack() {
    let content_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content");
    let app = evr_server::build_app(content_root).expect("base pack must load");
    assert!(app.world().contains_resource::<Content>());
}
