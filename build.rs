use std::{
    env,
    fs::{self, File},
    io::{self, BufWriter, Write},
};

// Take all files under `themes` and turn them into a file that contains a hashmap with their
// contents by name. This is pulled in theme.rs to construct themes.
fn main() -> io::Result<()> {
    let out_dir = env::var("OUT_DIR").unwrap();
    let output_path = format!("{out_dir}/themes.rs");
    let mut output_file = BufWriter::new(File::create(output_path)?);
    output_file.write_all(b"use std::collections::HashMap;\n")?;
    output_file.write_all(b"use once_cell::sync::Lazy;\n")?;
    output_file
        .write_all(b"static THEMES: Lazy<HashMap<&'static str, &'static [u8]>> = Lazy::new(|| HashMap::from([\n")?;
    for theme_file in fs::read_dir("themes")? {
        let theme_file = theme_file?;
        let metadata = theme_file.metadata()?;
        if !metadata.is_file() {
            panic!("found non file in themes directory");
        }
        let path = theme_file.path();
        let contents = fs::read(&path)?;
        let file_name = path.file_name().unwrap().to_string_lossy();
        let theme_name = file_name.split_once('.').unwrap().0;
        // TODO this wastes a bit of space
        output_file.write_all(format!("(\"{theme_name}\", {contents:?}.as_slice()),\n").as_bytes())?;

        // Rebuild if this theme changes.
        println!("cargo:rerun-if-changed={path:?}");
    }
    output_file.write_all(b"]));\n")?;
    Ok(())
}
