use chrono::prelude::*;
use log2::*;
use mime::Mime;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::error::Error;
use std::io::prelude::*;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::{fs, fs::File};
use teloxide::{net::Download, requests::Requester, types::Message, Bot};
use tera::Context;
use tera::Tera;
use tokio::fs::File as FileAsync;

use crate::utils::{copy_dir_all, get_folder_size, zip_folder};

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
    forward_date: String,
    text: String,
    photos: Vec<String>,
    videos: Vec<String>,
}

impl TelegramPost {
    async fn add_media(
        &mut self,
        bot: Bot,
        msg: Message,
        album_path: &Path,
        user_folder_size: u32,
        max_user_folder_size: u32,
    ) -> Result<(), Box<dyn Error>> {
        // Convert megabytes into bytes
        let max_user_folder_size = max_user_folder_size * 1024 * 1024;

        // Proceed if there is only one photo
        if let Some(photos) = msg.photo() {
            // Set post caption
            self.text = msg.caption().unwrap_or_default().to_string();

            // Find the largest photo by comparing their sizes
            let largest_photo = photos.iter().max_by_key(|photo| photo.width * photo.height);
            if let Some(photo) = largest_photo {
                // Photo file size shouldn't exceed 5 MB as stated in
                // https://core.telegram.org/bots/api#sending-files
                const MAX_PHOTO_FIZE_SIZE: u32 = 5 * 1024 * 1024;
                if photo.file.size > MAX_PHOTO_FIZE_SIZE {
                    error!(
                        "Cannot get photo file \"{}\" as it exceeds the size limit: {} > {}",
                        photo.file.id, photo.file.size, MAX_PHOTO_FIZE_SIZE
                    );
                    return Err("Photo file size exceeds 5 MB size limit!".into());
                }

                let new_user_folder_size = photo.file.size + user_folder_size;
                if new_user_folder_size > max_user_folder_size {
                    error!(
                        "User #{} folder has exceeded the size limit: {} > {}",
                        msg.from().unwrap().id.0,
                        new_user_folder_size,
                        max_user_folder_size
                    );
                    return Err("User folder has exceeded the size limit!".into());
                }
                match download_media_file(bot, album_path, &photo.file.id, "jpg").await {
                    Ok(file_name) => {
                        self.photos.push(file_name.to_string());
                    }
                    Err(_) => return Err("error downloading media file".into()),
                }
            }
        } else if let Some(video) = msg.video() {
            // Set post caption
            self.text = msg.caption().unwrap_or_default().to_string();

            // Only MP4 videos are supported at moment
            if let Some(ref mime_type) = video.mime_type {
                if mime_type == &Mime::from_str("video/mp4").unwrap() {
                    // Video file size shouldn't exceed 20 MB as stated in
                    // https://core.telegram.org/bots/api#sending-files
                    const MAX_VIDEO_FIZE_SIZE: u32 = 20 * 1024 * 1024;
                    if video.file.size > MAX_VIDEO_FIZE_SIZE {
                        error!(
                            "Cannot get video file \"{}\" as it exceeds the size limit: {} > {}",
                            video.file.id, video.file.size, MAX_VIDEO_FIZE_SIZE
                        );
                        return Err("Video file size exceeds 20 MB size limit!".into());
                    }

                    let new_user_folder_size = video.file.size + user_folder_size;
                    if new_user_folder_size > max_user_folder_size {
                        error!(
                            "User #{} folder has exceeded the size limit: {} > {}",
                            msg.from().unwrap().id.0,
                            new_user_folder_size,
                            max_user_folder_size
                        );
                        return Err("User folder has exceeded the size limit!".into());
                    }

                    match download_media_file(bot, album_path, &video.file.id, "mp4").await {
                        Ok(file_name) => {
                            self.videos.push(file_name.to_string());
                        }
                        Err(_) => return Err("error downloading media file".into()),
                    }
                }
            }
        }

        Ok(())
    }
}

fn parse_date(date_str: &str) -> DateTime<Utc> {
    let naive_date = NaiveDateTime::parse_from_str(date_str, "%Y-%m-%d %H:%M:%S UTC").unwrap();
    DateTime::<Utc>::from_naive_utc_and_offset(naive_date, Utc)
}

fn round_to_nearest_minute(date: DateTime<Utc>) -> DateTime<Utc> {
    let seconds = date.timestamp() % 60;
    date - chrono::Duration::seconds(seconds)
}

async fn create_html_file(
    album_folder: &Path,
    src_media_folder: &PathBuf,
    data: &str,
) -> Result<(), Box<dyn Error>> {
    let src_css = Path::new("templates").join("css");
    let src_img = Path::new("templates").join("img");

    let dst_css = album_folder.join("css");
    let dst_img = album_folder.join("img");
    let dst_media_folder = album_folder.join("gallery");

    // Copy the 'css' and 'img' folder
    copy_dir_all(&src_css, &dst_css)?;
    copy_dir_all(&src_img, &dst_img)?;

    // Copy the media folder
    copy_dir_all(src_media_folder, &dst_media_folder)?;

    let file_name = album_folder.join("index.html");
    fs::write(file_name, data)?;

    Ok(())
}

async fn download_media_file(
    bot: Bot,
    album_path: &Path,
    file_id: &String,
    file_extension: &str,
) -> Result<String, Box<dyn Error>> {
    let file_name = format!("{}.{}", file_id, file_extension);
    let file = bot.get_file(file_id).await?;
    fs::create_dir_all(album_path)?;
    let mut dst = FileAsync::create(&album_path.join(&file_name)).await?;
    bot.download_file(&file.path, &mut dst).await?;

    Ok(file_name)
}

pub async fn delete_user_folders(
    user_id: u64,
    data_folder: &str,
) -> Result<String, Box<dyn Error>> {
    let user_folder = Path::new(data_folder).join(user_id.to_string());

    if !user_folder.exists() {
        error!("No user data found for user #{}.", user_id);
        return Err("No data found!".into());
    }

    // Attempt to remove the specified folder and its contents
    match fs::remove_dir_all(user_folder) {
        Ok(_) => {
            info!("All user data for user #{} successfully deleted.", user_id);
        }
        Err(e) => {
            error!("Error deleting user data for user #{}: {}", user_id, e);
            return Err("error deleting data folder".into());
        }
    }

    Ok("All data deleted.".to_string())
}

pub async fn delete_user_album(
    username: String,
    user_id: u64,
    data_folder: &str,
) -> Result<String, Box<dyn Error>> {
    let file_path = Path::new(data_folder)
        .join(user_id.to_string())
        .join("data.json");
    let album_folder = Path::new(data_folder)
        .join(user_id.to_string())
        .join(&username);

    // Attempt to remove the specified folder and its contents
    match fs::remove_dir_all(album_folder) {
        Ok(_) => {
            info!(
                "Album \"{}\" for user #{} successfully deleted.",
                username, user_id
            );
        }
        Err(e) => {
            error!(
                "Error deleting album \"{}\" for user #{}: {}",
                username, user_id, e
            );
            return Err("error deleting album folder".into());
        }
    }

    // Read the file contents
    let mut file = File::open(&file_path)?;
    let mut json_data = String::new();
    file.read_to_string(&mut json_data)?;

    let mut telegram_data: TelegramData = serde_json::from_str(&json_data)?;

    if let Some(index) = telegram_data
        .channels
        .iter()
        .position(|channel| channel.username == username)
    {
        telegram_data.channels.remove(index);
    } else {
        error!("Album \"{}\" not found for user #{}", username, user_id);
        return Ok("Album not found.".to_string());
    }

    let updated_telegram_data = serde_json::to_string_pretty(&telegram_data)?;
    fs::write(&file_path, updated_telegram_data)?;
    info!(
        "Album \"{}\" for user #{} successfully deleted from JSON file.",
        username, user_id
    );

    Ok("Album deleted.".to_string())
}

pub async fn get_album_descriptions(
    user_id: u64,
    data_folder: &str,
) -> Result<Vec<String>, Box<dyn Error>> {
    // Read the file contents
    let file_path = Path::new(data_folder)
        .join(user_id.to_string())
        .join("data.json");
    let mut file = File::open(&file_path)?;
    let mut json_data = String::new();
    file.read_to_string(&mut json_data)?;

    let telegram_data: TelegramData = serde_json::from_str(&json_data)?;

    let mut channels_list: Vec<String> = Vec::new();
    for channel in telegram_data.channels.iter() {
        let channel_info = format!(
            "• <ins>{}</ins>\n{} ({} {})\n",
            channel.username,
            channel.title,
            channel.posts.len(),
            if channel.posts.len() == 1 {
                "post"
            } else {
                "posts"
            }
        );
        channels_list.push(channel_info);
    }

    if channels_list.is_empty() {
        return Err("no albums found".into());
    }

    Ok(channels_list)
}

pub async fn consolidate_media(user_id: u64, data_folder: &str) -> Result<String, Box<dyn Error>> {
    // Read the file contents
    let file_path = Path::new(data_folder)
        .join(user_id.to_string())
        .join("data.json");
    let mut file = File::open(&file_path)?;
    let mut json_data = String::new();
    file.read_to_string(&mut json_data)?;

    let mut telegram_data: TelegramData = serde_json::from_str(&json_data)?;

    if telegram_data.channels.is_empty() {
        return Err("no albums found".into());
    }

    // Consolidate posts with the same date and forward_date
    for channel in &mut telegram_data.channels {
        let mut similar_posts: HashMap<(DateTime<Utc>, DateTime<Utc>), Vec<&mut TelegramPost>> =
            HashMap::new();

        // Group posts
        for post in &mut channel.posts {
            let date = parse_date(&post.date);
            let forward_date = parse_date(&post.forward_date);

            let date_rounded = round_to_nearest_minute(date);
            let forward_date_rounded = round_to_nearest_minute(forward_date);

            similar_posts
                .entry((date_rounded, forward_date_rounded))
                .or_insert_with(Vec::new)
                .push(post);
        }

        // Replace posts with consolidated ones
        let mut updated_posts: Vec<TelegramPost> = similar_posts
            .into_iter()
            .flat_map(|(_, posts)| {
                posts.into_iter().fold(
                    None,
                    |acc: Option<TelegramPost>, post: &mut TelegramPost| match acc {
                        Some(mut updated_post) => {
                            updated_post.photos.extend_from_slice(&post.photos);
                            updated_post.videos.extend_from_slice(&post.videos);
                            if !post.text.is_empty() {
                                updated_post.text = post.text.clone();
                            }
                            Some(updated_post)
                        }
                        None => Some(post.clone()),
                    },
                )
            })
            .collect();

        // Sort posts by date
        updated_posts.sort_by(|a, b| a.date.cmp(&b.date));

        channel.posts = updated_posts;
    }

    let updated_telegram_data = serde_json::to_string_pretty(&telegram_data)?;
    fs::write(file_path, updated_telegram_data)?;
    info!(
        "Posts in all albums for user#{} have been successfully consolidated.",
        user_id
    );

    Ok("Posts in all albums have been successfully consolidated.".into())
}

async fn generate_single_album(
    tera: &Tera,
    channel: &TelegramChannel,
    user_id: u64,
    data_folder: &str,
    result_folder: &str,
) -> Result<(), Box<dyn Error>> {
    let mut context = Context::new();
    context.insert("channel", &channel);
    let data = tera.render("content.html", &context)?;
    let album_folder = Path::new(result_folder)
        .join(user_id.to_string())
        .join(&channel.username);
    let src_media_folder = Path::new(data_folder)
        .join(user_id.to_string())
        .join(&channel.username);
    create_html_file(&album_folder, &src_media_folder, &data).await?;

    Ok(())
}

pub async fn generate_albums(
    username: String,
    user_id: u64,
    data_folder: &str,
    result_folder: &str,
) -> Result<(u64, PathBuf), Box<dyn Error>> {
    // Read the file contents
    let file_path = Path::new(data_folder)
        .join(user_id.to_string())
        .join("data.json");
    let mut file = File::open(&file_path)?;
    let mut json_data = String::new();
    file.read_to_string(&mut json_data)?;

    let telegram_data: TelegramData = serde_json::from_str(&json_data)?;

    // Generate albums
    let mut counter: u64 = 0;
    let tera = Tera::new("templates/**/*.html")?;

    // Check if album exists
    let album_exists = telegram_data
        .channels
        .iter()
        .any(|channel| channel.username == username);
    if username != "<ALL>" && !album_exists {
        return Err("Album not found!".into());
    } else {
        for channel in telegram_data.channels.iter() {
            if username == "<ALL>" || username == channel.username {
                match generate_single_album(&tera, channel, user_id, data_folder, result_folder)
                    .await
                {
                    Ok(()) => {
                        info!(
                            "Successfully generated album \"{}\" for user #{}.",
                            username, user_id
                        );
                        counter += 1;
                    }
                    Err(e) => {
                        error!(
                            "Error generating album \"{}\" for user #{}: {}.",
                            username, user_id, e
                        );
                    }
                };

                if username == channel.username && username != "<ALL>" {
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
    let result_file = Path::new(result_folder).join(format!(
        "ArchiveGramBot-Archive-{}.zip",
        Utc::now().format("%Y-%m-%d_%H-%M-%S")
    ));

    // Safely use unwrap() here as amount of albums is > 0
    let album_zip = zip_folder(&user_folder, &result_file).unwrap();

    Ok((counter, album_zip))
}

pub async fn add_new_post(
    bot: Bot,
    msg: Message,
    data_folder: &str,
    max_user_folder_size: u32,
) -> Result<(), Box<dyn Error>> {
    let user_id = msg.chat.id.0 as u64;
    let album_id = msg.forward_from_chat().map_or(0, |chat| chat.id.0);
    let album_username = msg
        .forward_from_chat()
        .and_then(|chat| chat.username())
        .unwrap_or("(default)")
        .to_string();
    let post_id = msg.forward_from_message_id().unwrap_or(msg.id.0);

    let album_path = Path::new(data_folder)
        .join(user_id.to_string())
        .join(&album_username);

    let mut new_post = TelegramPost {
        id: post_id,
        date: msg.date.to_string(),
        forward_date: msg.forward_date().unwrap_or(msg.date).to_string(),
        text: msg.text().unwrap_or_default().to_string(),
        photos: vec![],
        videos: vec![],
    };

    let mut new_channel = TelegramChannel {
        id: album_id,
        title: msg
            .forward_from_chat()
            .and_then(|chat| chat.title())
            .unwrap_or("Default album")
            .to_string(),
        description: msg
            .forward_from_chat()
            .and_then(|chat| chat.description())
            .unwrap_or_default()
            .to_string(),
        username: album_username.clone(),
        posts: vec![],
    };

    // Read the file contents
    let user_folder = Path::new(data_folder).join(user_id.to_string());
    let file_path = user_folder.join("data.json");
    let user_folder_size = get_folder_size(&user_folder);

    if file_path.exists() {
        // If file exists, assume that it has correct format
        let mut file = File::open(&file_path)?;
        let mut json_data = String::new();
        file.read_to_string(&mut json_data)?;

        let mut telegram_data: TelegramData = serde_json::from_str(&json_data)?;

        // Album already exists
        if let Some(channel) = telegram_data
            .channels
            .iter_mut()
            .find(|channel| channel.id == album_id)
        {
            // Check if a post already exists
            if !channel.posts.iter().any(|post| post.id == post_id) {
                new_post
                    .add_media(
                        bot,
                        msg,
                        &album_path,
                        user_folder_size,
                        max_user_folder_size,
                    )
                    .await?;
                channel.posts.push(new_post);
                info!(
                    "Post #{} in album \"{}\" for user #{} successfully added to JSON file.",
                    post_id, album_username, user_id
                );
            } else {
                warn!(
                    "Post #{} already exists in album \"{}\" for user #{}.",
                    post_id, album_username, user_id
                );
                return Err("Post already exists!".into());
            }
        } else {
            // Album not found, add the new album to the list of albums
            new_post
                .add_media(
                    bot,
                    msg,
                    &album_path,
                    user_folder_size,
                    max_user_folder_size,
                )
                .await?;
            new_channel.posts.push(new_post);
            telegram_data.channels.push(new_channel);
            info!(
                "Post #{} and album \"{}\" for user #{} successfully added to JSON file.",
                post_id, album_username, user_id
            );
        }

        let updated_telegram_data = serde_json::to_string_pretty(&telegram_data)?;
        fs::write(&file_path, updated_telegram_data)?;
    } else {
        // Create a user dir if it doesn't exist
        fs::create_dir_all(&user_folder)?;

        // Create new JSON file for specified user
        new_post
            .add_media(
                bot,
                msg,
                &album_path,
                user_folder_size,
                max_user_folder_size,
            )
            .await?;
        new_channel.posts.push(new_post);
        let data = TelegramData {
            channels: vec![new_channel],
        };

        // Serialize the data to JSON
        let telegram_data = serde_json::to_string_pretty(&data)?;
        fs::write(&file_path, telegram_data)?;
        info!(
            "JSON file for user #{} created, post #{} and album \"{}\" successfully added.",
            user_id, post_id, album_username
        );
    }

    Ok(())
}
