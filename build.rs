use std::{fs, path::Path};

const MAC_PROTOS_PATH: &str =
    "/Applications/Anytype.app/Contents/Resources/app.asar.unpacked/dist/lib/protos/";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    fs::create_dir_all("protos")?;

    fs::read_dir(MAC_PROTOS_PATH)?.try_for_each(|result| {
        let dir_entry = result?;
        let file_name = dir_entry.file_name();
        let path = dir_entry.path();

        let file_content = fs::read_to_string(&path)?
            // TODO: Is there no way to tell protoc how to resolve these imports instead
            .replace("pkg/lib/pb/model/protos/models.proto", "models.proto")
            // TODO: This won't be necessary if a patch such as:
            // https://github.com/tokio-rs/prost/pull/506
            // was accepted
            .replace("oneof content", "oneof enum_content")
            .replace(
                "Metadata {\n    oneof payload",
                "Metadata {\n    oneof enum_payload",
            );
        fs::write(Path::new("protos").join(file_name), file_content)?;
        println!("cargo::rerun-if-changed={}", path.display());

        Ok::<(), std::io::Error>(())
    })?;

    tonic_build::configure().build_server(false).compile(
        &["protos/localstore.proto", "protos/models.proto"],
        &["protos"],
    )?;

    Ok(())
}
