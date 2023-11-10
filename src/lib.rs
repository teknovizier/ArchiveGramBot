use chrono::prelude::*;
use log2::*;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::{fs, fs::File};
use std::{io, io::prelude::*};
use std::path::{Path, PathBuf};
use tera::Context;
use tera::Tera;

pub mod utils;

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

fn create_html_file(album_folder: &PathBuf, src_media_folder: &PathBuf, data: &str) -> io::Result<()> {
    let src_css = Path::new("templates").join("css");
    let src_img = Path::new("templates").join("img");

    let dst_css = album_folder.join("css");
    let dst_img = album_folder.join("img");
    let dst_media_folder = album_folder.join("gallery");

    // Copy the 'css' and 'img' folder
    utils::copy_dir_all(&src_css, &dst_css)?;
    utils::copy_dir_all(&src_img, &dst_img)?;

    // Copy the media folder
    utils::copy_dir_all(&src_media_folder, &dst_media_folder)?;

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
    let album_zip = utils::zip_folder(&user_folder, &result_file).unwrap();

    Ok((counter, album_zip))
}
