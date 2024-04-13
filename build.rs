use std::{fs, path::Path};

const MAC_PROTOS_PATH: &str =
    "/Applications/Anytype.app/Contents/Resources/app.asar.unpacked/dist/lib/protos/";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    fs::create_dir_all("protos")?;

    fs::read_dir(MAC_PROTOS_PATH)?.try_for_each(|result| {
        let dir_entry = result?;
        let file_name = dir_entry.file_name();
        let path = dir_entry.path();

        fs::copy(&path, Path::new("protos").join(file_name))?;
        println!("cargo::rerun-if-changed={}", path.display());

        Ok::<(), std::io::Error>(())
    })?;

    Ok(())
}
