use chrono::prelude::*;
use log2::*;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::{fs, fs::File};
use std::{io, io::prelude::*};
use std::path::{Path, PathBuf};
use teloxide::types::Message;
use tera::Context;
use tera::Tera;

pub mod utils;

#[derive(Debug, Deserialize, Serialize)]
pub struct TelegramData {
    channels: Vec<TelegramChannel>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct TelegramChannel {
    id: i64,
    title: String,
    description: String,
    username: String,
    posts: Vec<TelegramPost>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct TelegramPost {
    id: i32,
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

pub async fn delete_user_folders(user_id: u64, data_folder: &str) -> Result<(), Box<dyn Error>> {
    let user_folder = Path::new(data_folder).join(user_id.to_string());

    // Attempt to remove the specified folder and its contents
    match fs::remove_dir_all(user_folder) {
        Ok(_) => {
            info!("All user data for user {} successfully deleted.", user_id);
        },
        Err(e) => {
            error!("Error deleting user data for user {}: {}", user_id, e);
            return Err("error deleting data. Please contact bot owners".into())
        } 
    }

    Ok(())
}

pub async fn delete_user_album(album_id: i64, user_id: u64, data_folder: &str) -> Result<(), Box<dyn Error>> {
    let file_path = Path::new(data_folder).join(user_id.to_string()).join("data.json");
    let album_folder = Path::new(data_folder).join(user_id.to_string()).join(album_id.to_string());

    // Attempt to remove the specified folder and its contents
    match fs::remove_dir_all(album_folder) {
        Ok(_) => {
            info!("Album #{} for user {} successfully deleted.", album_id, user_id);
        },
        Err(e) => {
            error!("Error deleting album #{} for user {}: {}", album_id, user_id, e);
            return Err("error deleting album. Please check album ID and/or contact bot owners".into())
        } 
    }

    // Read the file contents
    let mut file = File::open(format!("{}/{}/data.json", data_folder, user_id.to_string()))?;
    let mut json_data = String::new();
    file.read_to_string(&mut json_data)?;

    let mut telegram_data: TelegramData = serde_json::from_str(&json_data)?;

    if let Some(index) = telegram_data.channels.iter().position(|channel| channel.id == album_id) {
        telegram_data.channels.remove(index);
        info!("Album #{} for user {} successfully deleted from JSON file.", album_id, user_id);
    }
    else {
        error!("Error deleting album #{} info from JSON file for user {}", album_id, user_id);
        return Err("album not found?".into());
    }

    let updated_telegram_data = serde_json::to_string_pretty(&telegram_data)?;
    fs::write(file_path, updated_telegram_data)?;

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
        let channel_info = format!("{}) {} ({} posts)", channel.id.to_string(), channel.title, channel.posts.len().to_string());
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

pub async fn generate_albums(album_id: i64, user_id: u64, data_folder: &str, result_folder: &str) -> Result<(u64, PathBuf), Box<dyn Error>> {
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

pub async fn add_new_post(msg: Message, data_folder: &str) -> Result<(), Box<dyn Error>> {
    let user_id = msg.chat.id.0 as u64;
    let album_id = msg.forward_from_chat().map_or(0, |chat| chat.id.0 as i64);
    let post_id = msg.forward_from_message_id().unwrap_or(msg.id.0 as i32);

    let new_post = TelegramPost {
        id: post_id,
        date: msg.date.to_string(),
        text: {
            if let Some(_) = msg.photo() {
                msg.caption().unwrap_or_default().to_string()
            }
            else {
                msg.text().unwrap_or_default().to_string()
            }
        },
        photos: [].to_vec(),
        videos: [].to_vec()
    };

    let new_channel = TelegramChannel {
        id: album_id,
        title: msg.forward_from_chat().and_then(|chat| chat.title()).unwrap_or_default().to_string(),
        description: msg.forward_from_chat().and_then(|chat| chat.description()).unwrap_or_default().to_string(),
        username: msg.forward_from_chat().and_then(|chat| chat.username()).unwrap_or_default().to_string(),
        posts: vec![new_post.clone()],
    };

    // Read the file contents
    let user_path = Path::new(data_folder).join(user_id.to_string());
    let file_path = user_path.join("data.json");

    if file_path.exists() {
        // If file exists, assume that it has correct format
        let mut file = File::open(&file_path)?;
        let mut json_data = String::new();
        file.read_to_string(&mut json_data)?;

        let mut telegram_data: TelegramData = serde_json::from_str(&json_data)?;

        // Album already exists
        if let Some(channel) = telegram_data.channels.iter_mut().find(|channel| channel.id == album_id) {
            // Check if a post already exists
            if !channel.posts.iter().any(|post| post.id == post_id) {
                channel.posts.push(new_post);
                info!("Post #{} in album #{} for user {} successfully added to JSON file.", post_id, album_id, user_id);
            } else {
                warn!("Post #{} already exists in album #{} for user {}.", post_id, album_id, user_id);
                return Err("post already exists".into())
            }
        }
        else {
            // Album not found, add the new album to the list of albums
            telegram_data.channels.push(new_channel);
            info!("Post #{} and album #{} for user {} successfully added to JSON file.", post_id, album_id, user_id);
        }

        let updated_telegram_data = serde_json::to_string_pretty(&telegram_data)?;
        fs::write(&file_path, updated_telegram_data)?;
    }
    else {
        // Create a user dir if it doesn't exist
        fs::create_dir_all(&user_path)?;

        // Create new JSON file for specified user
        let data = TelegramData {
            channels: vec![new_channel],
        };

        // Serialize the data to JSON
        let telegram_data = serde_json::to_string_pretty(&data)?;
        fs::write(&file_path, telegram_data)?;
        info!("JSON file for user {} created, post #{} and album #{} successfully added.", user_id, post_id, album_id);
    }

    Ok(())
}

