use std::{
    env,
    fs::{self, File},
    io::{self, BufWriter, Write},
};

// Take all files under `themes` and turn them into a file that contains a hashmap with their
// contents by name. This is pulled in theme.rs to construct themes.
fn build_themes(out_dir: &str) -> io::Result<()> {
    let output_path = format!("{out_dir}/themes.rs");
    let mut output_file = BufWriter::new(File::create(output_path)?);
    output_file.write_all(b"use std::collections::BTreeMap as Map;\n")?;
    output_file.write_all(b"use once_cell::sync::Lazy;\n")?;
    output_file.write_all(b"static THEMES: Lazy<Map<&'static str, &'static [u8]>> = Lazy::new(|| Map::from([\n")?;

    let mut paths = fs::read_dir("themes")?.collect::<io::Result<Vec<_>>>()?;
    paths.sort_by_key(|e| e.path());
    for theme_file in paths {
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
    }
    output_file.write_all(b"]));\n")?;

    // Rebuild if anything changes.
    println!("cargo:rerun-if-changed=themes");
    Ok(())
}

fn build_executors(out_dir: &str) -> io::Result<()> {
    let output_path = format!("{out_dir}/executors.rs");
    let mut output_file = BufWriter::new(File::create(output_path)?);
    output_file.write_all(b"use std::collections::BTreeMap as Map;\n")?;
    output_file.write_all(b"use once_cell::sync::Lazy;\n")?;
    output_file.write_all(b"static EXECUTORS: Lazy<Map<crate::markdown::elements::CodeLanguage, &'static [u8]>> = Lazy::new(|| Map::from([\n")?;

    let mut paths = fs::read_dir("executors")?.collect::<io::Result<Vec<_>>>()?;
    paths.sort_by_key(|e| e.path());
    for file in paths {
        let metadata = file.metadata()?;
        if !metadata.is_file() {
            panic!("found non file in executors directory");
        }
        let path = file.path();
        let contents = fs::read(&path)?;
        let file_name = path.file_name().unwrap().to_string_lossy();
        let (executor_name, extension) = file_name.split_once('.').unwrap();
        if extension != "sh" {
            panic!("extension must be 'sh'");
        }
        output_file.write_all(
            format!("(crate::markdown::elements::CodeLanguage::{executor_name}, {contents:?}.as_slice()),\n")
                .as_bytes(),
        )?;
    }
    output_file.write_all(b"]));\n")?;

    // Rebuild if anything changes.
    println!("cargo:rerun-if-changed=executors");
    Ok(())
}

fn main() -> io::Result<()> {
    let out_dir = env::var("OUT_DIR").unwrap();
    build_themes(&out_dir)?;
    build_executors(&out_dir)?;
    Ok(())
}
