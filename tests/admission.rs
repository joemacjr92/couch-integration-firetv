//! Exercise the exact production executable through Couch's package host.
//!
//! Package::host() starts the compiled binary and completes the real Hello
//! handshake. That handshake rejects any difference between the manifest
//! embedded in the executable and the manifest supplied by the package.

use couch_plugin::testing::{Adapter, Package};
use std::path::Path;

#[test]
#[ignore = "runs in GitHub Actions; local WSL permissions prevent execution"]
fn executable_manifest_matches_the_package_manifest() {
    let package = Package::new(Adapter {
        binary: Path::new(env!("CARGO_BIN_EXE_couch-plugin-firetv")),
        manifest_json: include_str!("../plugin.json"),
    });

    // Spawning the host performs the production manifest/protocol handshake.
    // A mismatch fails here before an APK can be built or published.
    let _host = package.host();
}