use std::{fs, fs::File};
use std::{io, io::prelude::*};
use std::path::{Path, PathBuf};
use zip::CompressionMethod;
use zip::write::FileOptions;

pub fn copy_dir_all(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> io::Result<()> {
    fs::create_dir_all(&dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        if ty.is_dir() {
            copy_dir_all(entry.path(), dst.as_ref().join(entry.file_name()))?;
        } else {
            fs::copy(entry.path(), dst.as_ref().join(entry.file_name()))?;
        }
    }
    Ok(())
}

pub fn zip_folder(folder_path: &PathBuf, result_file: &PathBuf) -> Result<PathBuf, Box<dyn std::error::Error>> {
    // Create a zip file
    let file = File::create(&result_file)?;
    let mut zip = zip::ZipWriter::new(file);

    // Walk through the files in the folder
    let options = FileOptions::default()
        .compression_method(CompressionMethod::Stored);
        
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
