use log2::*;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::{fs, fs::File};
use std::{io, io::prelude::*};
use std::path::{Path, PathBuf};
use tera::Context;
use tera::Tera;

#[derive(Debug, Deserialize, Serialize)]
pub struct TelegramData {
    channels: Vec<TelegramChannel>,
}

#[derive(Debug, Deserialize, Serialize)]
struct TelegramChannel {
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

pub fn generate_albums(user_id: u32, data_folder: &str, result_folder: &str) -> Result<u32, Box<dyn Error>> {
    // Read the file contents
    let mut file = File::open(format!("{}/{}/data.json", data_folder, user_id.to_string()))?;
    let mut json_data = String::new();
    file.read_to_string(&mut json_data)?;

    let telegram_data: TelegramData = serde_json::from_str(&json_data)?;
    
    // Generate albums
    let mut counter = 0;
    let tera = Tera::new("templates/**/*.html")?;
    for channel in telegram_data.channels.iter() {
        let mut context = Context::new();
        context.insert("channel", &channel);
        let data = tera.render("content.html", &context)?;
        let album_folder = Path::new(result_folder).join(user_id.to_string()).join(channel.id.to_string());
        let src_media_folder = Path::new(data_folder).join(user_id.to_string()).join(channel.id.to_string());
        create_html_file(&album_folder, &src_media_folder, &data)?;
        counter += 1;
        info!("Successfully generated album #{}.", channel.id);
    }

    Ok(counter)
}
