//! Couch subprocess entry point. stdout belongs exclusively to the framed
//! plugin protocol; do not add println!/logging here.
fn main() {
    let manifest = serde_json::from_str(include_str!("../../plugin.json"))
        .expect("embedded Fire TV integration manifest");
    if couch_plugin::serve::<couch_firetv::FireTvClient>(manifest).is_err() {
        std::process::exit(1);
    }
}
