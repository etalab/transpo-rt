extern crate prost_build;

fn main() {
    let mut config = prost_build::Config::new();
    config.type_attribute(".", "#[derive(Serialize)]");
    config.type_attribute(".", "#[serde(rename_all = \"camelCase\")]");

    config
        .compile_protos(&["proto/gtfs-realtime.proto"], &["proto/"])
        .unwrap();
}
