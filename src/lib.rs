use chrono::prelude::*;
use log2::*;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::{fs, fs::File};
use std::{io, io::prelude::*};
use std::path::{Path, PathBuf};
use tera::Context;
use tera::Tera;
use zip::CompressionMethod;
use zip::write::FileOptions;

#[derive(Debug, Deserialize, Serialize)]
pub struct TelegramData {
    channels: Vec<TelegramChannel>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct TelegramChannel {
    id: u64,
    name: String,
    desc: String,
    url: String,
    posts: Vec<TelegramPost>,
}

#[derive(Debug, Deserialize, Serialize)]
struct TelegramPost {
    id: u64,
    date: String,
    text: String,
    photos: Vec<String>,
    videos: Vec<String>,
}

fn copy_dir_all(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> io::Result<()> {
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

fn zip_folder(folder_path: &PathBuf, result_file: &PathBuf) -> Result<PathBuf, Box<dyn std::error::Error>> {
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

fn create_html_file(album_folder: &PathBuf, src_media_folder: &PathBuf, data: &str) -> io::Result<()> {
    let src_css = Path::new("templates").join("css");
    let src_img = Path::new("templates").join("img");

    let dst_css = album_folder.join("css");
    let dst_img = album_folder.join("img");
    let dst_media_folder = album_folder.join("gallery");

    // Copy the 'css' and 'img' folder
    copy_dir_all(&src_css, &dst_css)?;
    copy_dir_all(&src_img, &dst_img)?;

    // Copy the media folder
    copy_dir_all(&src_media_folder, &dst_media_folder)?;

    let file_name = album_folder.join("index.html");
    fs::write(&file_name, &data)?;

    Ok(())
}

pub async fn get_album_descriptions(user_id: u64, data_folder: &str) -> Result<Vec<String>, Box<dyn Error>> {
    // Read the file contents
    let mut file = File::open(format!("{}/{}/data.json", data_folder, user_id.to_string()))?;
    let mut json_data = String::new();
    file.read_to_string(&mut json_data)?;

    let telegram_data: TelegramData = serde_json::from_str(&json_data)?;
    
    let mut channels_list: Vec<String> = Vec::new();
    for channel in telegram_data.channels.iter() {
        let channel_info = format!("{}) {} ({} posts)", channel.id.to_string(), channel.name, channel.posts.len().to_string());
        channels_list.push(channel_info);
    }

    if channels_list.is_empty() {
        error!("No albums found for user for user {}!", user_id);
        return Err("no albums found".into());
    }

    Ok(channels_list)
}

fn generate_single_album(tera: &Tera, channel: &TelegramChannel, user_id: u64, data_folder: &str, result_folder: &str) -> Result<(), Box<dyn Error>> {
    let mut context = Context::new();
    context.insert("channel", &channel);
    let data = tera.render("content.html", &context)?;
    let album_folder = Path::new(result_folder).join(user_id.to_string()).join(channel.id.to_string());
    let src_media_folder = Path::new(data_folder).join(user_id.to_string()).join(channel.id.to_string());
    create_html_file(&album_folder, &src_media_folder, &data)?;

    Ok(())
}

pub async fn generate_albums(album_id: u64, user_id: u64, data_folder: &str, result_folder: &str) -> Result<(u64, PathBuf), Box<dyn Error>> {
    // Read the file contents
    let mut file = File::open(format!("{}/{}/data.json", data_folder, user_id.to_string()))?;
    let mut json_data = String::new();
    file.read_to_string(&mut json_data)?;

    let telegram_data: TelegramData = serde_json::from_str(&json_data)?;
    
    // Generate albums
    let mut counter: u64 = 0;
    let tera = Tera::new("templates/**/*.html")?;

    // Check if album exists
    let album_exists = &telegram_data.channels.iter().any(|channel| channel.id == album_id);
    if album_id != 0 && !(*album_exists) {
        error!("Album #{} doesn't exist!", album_id);
    }
    else {
        for channel in telegram_data.channels.iter() {
            if album_id == 0 || album_id == channel.id {
                match generate_single_album(&tera, channel, user_id, data_folder, result_folder) {
                    Ok(()) => {
                        info!("Successfully generated album #{} for user {}.", channel.id, user_id);
                        counter += 1;
                    },
                    Err(e) => {
                        error!("Error generating album #{} for user {}: {}.", channel.id, user_id, e);
                    }
                };

                if album_id == channel.id {
                    // If the required album is generated, break the loop
                    break;
                }
            }
        }
    }

    if counter == 0 {
        // Return an error if counter is 0
        return Err("no albums have been generated".into());
    }

    let user_folder = Path::new(result_folder).join(user_id.to_string());
    let result_file = Path::new(result_folder).join(format!("ArchiveGramBot-Archive-{}.zip", Utc::now().format("%Y-%m-%d_%H-%M-%S")));

    // Safely use unwrap() here as amount of albums is > 0
    let album_zip = zip_folder(&user_folder, &result_file).unwrap();

    Ok((counter, album_zip))
}
