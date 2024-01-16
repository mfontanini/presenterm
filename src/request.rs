use reqwest::blocking::get;
use std::{
    fs::{self, File},
    io::copy,
    path::Path,
};

const DEMO_MD_URL: &str = "https://raw.githubusercontent.com/mfontanini/presenterm/master/examples/demo.md";
const DEMO_JPG_URL: &str = "https://raw.githubusercontent.com/mfontanini/presenterm/master/examples/doge.png";

pub fn run_demo(temp_dir_root: &Path) -> Result<(), Box<dyn std::error::Error>> {
    if !temp_dir_root.exists() {
        fs::create_dir_all(temp_dir_root)?;
    }

    download_file_to_temp(DEMO_MD_URL, temp_dir_root, "demo.md")?;
    download_file_to_temp(DEMO_JPG_URL, temp_dir_root, "doge.png")?;
    Ok(())
}

/// Downloads a file from the specified URL and saves it to the specified temporary directory.
///
/// # Arguments
///
/// * `url` - The URL of the file to download.
/// * `temp_dir_root` - The root path of the temporary directory where the file will be saved.
/// * `file_name` - The name of the file to be saved in the temporary directory.
///
/// # Returns
///
/// Returns `Ok(())` if the file is successfully downloaded and saved, otherwise returns an
/// `Err` containing an error message.
///
/// # Errors
///
/// Returns an error on followings
///
/// * Unable to make a successful HTTP request to the specified URL.
/// * The HTTP response status is not a success status code.
/// * Unable to create the specified file in the temporary directory.
/// * Unable to write the downloaded content to the file.
pub fn download_file_to_temp(
    url: &str,
    temp_dir_root: &Path,
    file_name: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let file_path = temp_dir_root.join(file_name);

    let mut response = get(url)?;

    if !response.status().is_success() {
        return Err(format!("Unable to get file: Request failed with status code {}", response.status()).into());
    }

    let mut file = File::create(&file_path)?;

    copy(&mut response, &mut file)?;

    println!("Info: Downloaded - {}", file_path.display());

    Ok(())
}
