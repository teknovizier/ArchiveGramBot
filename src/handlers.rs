use log2::*;
use regex::Regex;
use std::fs;
use std::path::{Path, PathBuf};
use teloxide::{prelude::*, types::InputFile, types::ParseMode, utils::command::BotCommands};
type HandlerResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;

use crate::operations::{
    add_new_post, consolidate_media, delete_user_album, delete_user_folders, generate_albums,
    get_album_descriptions,
};
use crate::utils::{convert_to_mb, delete_contents_of_folder, get_folder_size, Config};

#[derive(BotCommands, Clone)]
#[command(
    rename_rule = "lowercase",
    description = "These commands are supported:"
)]
pub enum Command {
    #[command(description = "display this text.")]
    Help,
    #[command(description = "show identifiers and names for all available albums.")]
    ShowAlbums,
    #[command(
        description = "if the original posts contain multiple media files, consolidate them. Action is performed for all albums"
    )]
    ConsolidateAll,
    #[command(description = "generate all albums.")]
    GenerateAll,
    #[command(
        description = "generate specified album (add album `username` after `generate` command)."
    )]
    Generate(String),
    #[command(description = "delete all albums.")]
    DeleteAll,
    #[command(
        description = "delete specified album (add album `username` after `delete` command)."
    )]
    Delete(String),
}

pub async fn help(bot: Bot, msg: Message) -> HandlerResult {
    bot.send_message(msg.chat.id, Command::descriptions().to_string())
        .await?;
    Ok(())
}

pub async fn showalbums(bot: Bot, msg: Message, config: &Config) -> HandlerResult {
    let user_id = msg.chat.id.0 as u64;
    let mut albums: Option<Vec<String>> = None;
    match get_album_descriptions(user_id, &config.data_folder).await {
        Ok(a) => {
            albums = Some(a);
        }
        Err(err) => {
            error!("showalbums(): user #{}: {}", user_id, err);
        }
    }

    if let Some(albums) = albums {
        let user_folder = Path::new(&config.data_folder).join(user_id.to_string());
        let user_folder_size_in_mb = convert_to_mb(get_folder_size(&user_folder));

        bot.send_message(
            msg.chat.id,
            format!(
                "<strong>Available albums</strong>:\n\n{}\n<strong>Total occupied space:</strong> {}/{} MB",
                albums.join("\n"),
                user_folder_size_in_mb,
                config.max_user_folder_size
            ),
        )
        .parse_mode(ParseMode::Html)
        .await?;
    } else {
        bot.send_message(msg.chat.id, "❗ No albums found!").await?;
    }

    Ok(())
}

pub async fn consolidateall(bot: Bot, msg: Message, config: &Config) -> HandlerResult {
    let user_id = msg.chat.id.0 as u64;
    let mut ok_string: Option<String> = None;

    match consolidate_media(user_id, &config.data_folder).await {
        Ok(res) => {
            ok_string = Some(res);
        }
        Err(err) => {
            error!("consolidateall(): user #{}: {}", user_id, err);
        }
    }

    if let Some(message) = ok_string {
        bot.send_message(msg.chat.id, format!("✅ {}", message))
            .await?;
    } else {
        bot.send_message(msg.chat.id, "❗ No albums found!".to_string())
            .await?;
    }

    Ok(())
}

pub async fn generateall(bot: Bot, msg: Message, config: &Config) -> HandlerResult {
    let mut counter: Option<u64> = None;
    let mut zip_file: Option<PathBuf> = None;

    // Assume that user ID is the same as chat ID
    let user_id = msg.chat.id.0 as u64;

    // Generate all albums
    match generate_albums(
        "<ALL>".to_string(),
        user_id,
        &config.data_folder,
        &config.result_folder,
    )
    .await
    {
        Ok(c) => {
            counter = Some(c.0);
            zip_file = Some(c.1);
        }
        Err(err) => {
            error!("generateall(): user #{}: {}", user_id, err);
        }
    }

    if let Some(counter) = counter {
        let success_msg = bot
            .send_message(
                msg.chat.id,
                format!("✅ Successfully generated {} albums.", counter),
            )
            .await?;
        let zip_path = zip_file.unwrap();
        // Do not try to send an archive that exceed 20 MB
        if fs::metadata(&zip_path).unwrap().len() > 20 * 1024 * 1024 {
            warn!("An archive with all albums requested by user #{} exceeds 20 MB size limit and hasn't been sent", user_id);
            bot.send_message(msg.chat.id, "❗ Archive size exceeds 20 MB and cannot be sent automatically. In order to get it, please contact bot owners.").reply_to_message_id(success_msg.id).await?;
        } else {
            let waiting_msg = bot.send_message(msg.chat.id, "⌛️").await?;
            let input_file = InputFile::file(&zip_path);
            bot.send_document(msg.chat.id, input_file)
                .reply_to_message_id(success_msg.id)
                .await?;
            bot.delete_message(msg.chat.id, waiting_msg.id).await?;
            info!("Sent an archive with all albums to user #{}", user_id);
            delete_contents_of_folder(&config.result_folder).await?;
        }
    } else {
        bot.send_message(msg.chat.id, "❗ Error generating albums!")
            .await?;
    }

    Ok(())
}

pub async fn generate(bot: Bot, msg: Message, config: &Config, username: String) -> HandlerResult {
    let mut counter: Option<u64> = None;
    let mut zip_file: Option<PathBuf> = None;
    let mut error_string = String::new();

    // Assume that user ID is the same as chat ID
    let user_id = msg.chat.id.0 as u64;

    // Generate single album
    match generate_albums(
        username.clone(),
        user_id,
        &config.data_folder,
        &config.result_folder,
    )
    .await
    {
        Ok(c) => {
            counter = Some(c.0);
            zip_file = Some(c.1);
        }
        Err(err) => {
            error!("generate(): user #{}: {}", user_id, err);
            error_string = err.to_string();
        }
    }

    if counter.is_some() {
        let success_msg = bot
            .send_message(
                msg.chat.id,
                format!("✅ Successfully generated album \"{}\".", username),
            )
            .await?;
        let zip_path = zip_file.unwrap();
        // Do not try to send an archive that exceed 20 MB
        if fs::metadata(&zip_path).unwrap().len() > 20 * 1024 * 1024 {
            warn!("An archive with all albums requested by user #{} exceeds 20 MB size limit and hasn't been sent", user_id);
            bot.send_message(msg.chat.id, "❗ Archive size exceeds 20 MB and cannot be sent automatically. In order to get it, please contact bot owners.".to_string()).reply_to_message_id(success_msg.id).await?;
        } else {
            let waiting_msg = bot.send_message(msg.chat.id, "⌛️").await?;
            let input_file = InputFile::file(&zip_path);
            bot.send_document(msg.chat.id, input_file)
                .reply_to_message_id(success_msg.id)
                .await?;
            bot.delete_message(msg.chat.id, waiting_msg.id).await?;
            info!(
                "Sent an archive with album \"{}\" to user #{}",
                username, user_id
            );
            delete_contents_of_folder(&config.result_folder).await?;
        }
    } else if error_string == "Album not found!" {
        bot.send_message(msg.chat.id, format!("❌ {}", error_string))
            .await?;
    } else {
        bot.send_message(
            msg.chat.id,
            format!("❌ Error generating album \"{}\"!", username),
        )
        .await?;
    }

    Ok(())
}

pub async fn deleteall(bot: Bot, msg: Message, config: &Config) -> HandlerResult {
    let user_id = msg.chat.id.0 as u64;
    let mut ok_string: Option<String> = None;
    let mut error_string = String::new();

    match delete_user_folders(user_id, &config.data_folder).await {
        Ok(res) => {
            ok_string = Some(res);
        }
        Err(err) => {
            error!("deleteall(): user #{}: {}", user_id, err);
            error_string = err.to_string();
        }
    }

    if let Some(message) = ok_string {
        bot.send_message(msg.chat.id, format!("✅ {}", message))
            .await?;
    } else if error_string == "No data found!" {
        bot.send_message(msg.chat.id, format!("❗ {}", error_string))
            .await?;
    } else {
        bot.send_message(
            msg.chat.id,
            "❌ Error deleting data. Please contact bot owners!",
        )
        .await?;
    }

    Ok(())
}

pub async fn delete(bot: Bot, msg: Message, config: &Config, username: String) -> HandlerResult {
    let user_id = msg.chat.id.0 as u64;
    let mut ok_string: Option<String> = None;

    match delete_user_album(username, user_id, &config.data_folder).await {
        Ok(res) => {
            ok_string = Some(res);
        }
        Err(err) => {
            error!("delete(): user #{}: {}", user_id, err);
        }
    }

    if let Some(message) = ok_string {
        bot.send_message(msg.chat.id, format!("✅ {}", message))
            .await?;
    } else {
        bot.send_message(
            msg.chat.id,
            "❌ Error deleting album. Please check album username and/or contact bot owners!"
                .to_string(),
        )
        .await?;
    }

    Ok(())
}

pub async fn reply_not_authorized(bot: Bot, msg: Message) -> HandlerResult {
    bot.send_message(msg.chat.id, "❗ You are not authorized to use this bot.")
        .await?;
    Ok(())
}

pub async fn reply(bot: Bot, msg: Message, config: &Config) -> HandlerResult {
    if let Some(text) = msg.text() {
        if text == "/start" {
            return Ok(());
        } else if text.starts_with('/') {
            bot.send_message(
                msg.chat.id,
                "❌ Invalid command! Please call /help to see the list of available commands."
                    .to_string(),
            )
            .await?;
            return Ok(());
        }
    }

    let waiting_msg = bot.send_message(msg.chat.id, "⌛️").await?;

    let chat_id = msg.chat.id;
    let msg_id = msg.id;
    let mut ok_string: Option<&str> = None;
    let mut error_string = String::new();

    match add_new_post(
        bot.clone(),
        msg,
        &config.data_folder,
        config.max_user_folder_size,
    )
    .await
    {
        Ok(_) => {
            ok_string = Some("Message added to archive.");
        }
        Err(err) => {
            error!("reply(): user #{}: {}", chat_id, err);
            error_string = err.to_string();
        }
    }

    bot.delete_message(chat_id, waiting_msg.id).await?;
    if let Some(message) = ok_string {
        bot.send_message(chat_id, format!("✅ {}", message))
            .reply_to_message_id(msg_id)
            .await?;
    } else {
        let error_strings: Vec<String> = vec![
            "Post already exists!".to_string(),
            "User folder cannot exceed \\d+ MB size limit!".to_string(),
            "Photo file size exceeds \\d+ MB size limit!".to_string(),
            "Video file size exceeds \\d+ MB size limit!".to_string(),
        ];

        let mut found = false;
        for pattern in &error_strings {
            let re = Regex::new(&format!("^{}$", pattern)).unwrap();
            if re.is_match(&error_string) {
                found = true;
                break;
            }
        }

        if found {
            // Known error message
            bot.send_message(chat_id, format!("❗ {}", error_string))
                .reply_to_message_id(msg_id)
                .await?;
        } else {
            // Unknown error message
            bot.send_message(
                chat_id,
                "❌ Error adding message! Please contact bot owners!".to_string(),
            )
            .reply_to_message_id(msg_id)
            .await?;
        }
    }

    Ok(())
}
