use serde::Deserialize;
use std::path::{Path, PathBuf};
use std::{fs, fs::File};
use std::{io, io::prelude::*};
use zip::write::FileOptions;
use zip::CompressionMethod;

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub teloxide_token: String,
    pub data_folder: String,
    pub max_user_folder_size: u32,
    pub result_folder: String,
    pub log_path: String,
    pub restrict_access: bool,
    pub allowed_users: Vec<u64>,
}

pub fn load_config(file: &str) -> Config {
    let contents = match fs::read_to_string(file) {
        Ok(_a) => _a,
        Err(_) => panic!("Could not read file \"{}\"", file),
    };

    let config: Config = match toml::from_str(&contents) {
        Ok(_b) => _b,
        Err(_) => panic!("Unable to load data from \"{}\"", file),
    };

    config
}

pub fn get_folder_size(folder_path: &Path) -> u32 {
    let mut total_size: u32 = 0;

    for entry in walkdir::WalkDir::new(folder_path)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if entry.file_type().is_file() {
            total_size += entry.metadata().map_or(0, |m| m.len()) as u32;
        }
    }

    total_size
}

pub fn convert_to_mb(bytes: u32) -> f64 {
    (bytes as f64 / (1024.0 * 1024.0) * 100.0).round() / 100.0
}

pub fn copy_dir_all(src: &PathBuf, dst: &PathBuf) -> io::Result<()> {
    if src.exists() && src.is_dir() {
        fs::create_dir_all(dst)?;
        for entry in fs::read_dir(src)? {
            let entry = entry?;
            let ty = entry.file_type()?;
            if ty.is_dir() {
                copy_dir_all(&entry.path(), &dst.join(entry.file_name()))?;
            } else {
                fs::copy(entry.path(), dst.join(entry.file_name()))?;
            }
        }
    }

    Ok(())
}

pub fn truncate_string(s: &str, max_length: usize) -> String {
    if s.chars().count() > max_length {
        s.chars().take(max_length).collect::<String>() + "..."
    } else {
        s.to_string()
    }
}

pub fn zip_folder(
    folder_path: &PathBuf,
    result_file: &PathBuf,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    // Create a zip file
    let file = File::create(result_file)?;
    let mut zip = zip::ZipWriter::new(file);

    // Walk through the files in the folder
    let options = FileOptions::default().compression_method(CompressionMethod::Stored);

    for entry in walkdir::WalkDir::new(folder_path) {
        let entry = entry?;
        let relative_path = entry.path().strip_prefix(folder_path)?;

        if entry.file_type().is_file() {
            // Add each file to the zip archive
            zip.start_file(relative_path.to_str().unwrap(), options)?;
            let mut file = File::open(entry.path())?;
            let mut buffer = Vec::new();
            file.read_to_end(&mut buffer)?;
            zip.write_all(&buffer)?;
        }
    }

    Ok(result_file.clone())
}

pub async fn delete_contents_of_folder(folder_path: &str) -> io::Result<()> {
    // Convert the folder path to a Path
    let path = Path::new(folder_path);

    // Iterate over the contents of the folder
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let entry_path = entry.path();

        // Remove files or directories
        if entry_path.is_file() {
            fs::remove_file(entry_path)?;
        } else if entry_path.is_dir() {
            fs::remove_dir_all(entry_path)?;
        }
    }

    Ok(())
}
