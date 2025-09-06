use std::path::Path;

fn main() {
    let out_dir = Path::new("src/protos");

    prost_build::Config::new()
        .out_dir(out_dir)
        .compile_protos(
            &["protos/SophonManifestProto.proto", "protos/SophonPatchProto.proto"],
            &["protos"],
        )
        .unwrap();
}
