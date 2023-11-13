use log2::*;
use serde::Deserialize;
use std::fs;
use std::path::PathBuf;
use teloxide::{prelude::*, utils::command::BotCommands, types::InputFile};
use archivegrambot as agb;
type HandlerResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;

pub mod utils;

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase", description = "These commands are supported:")]
enum Command {
    #[command(description = "display this text.")]
    Help,
    #[command(description = "show identifiers and names for all available albums.")]
    ShowAlbums,
    #[command(description = "generate all albums.")]
    GenerateAll,
    #[command(description = "generate specified album (add album ID after `generate` command).")]
    Generate(i64),
    #[command(description = "delete all albums.")]
    DeleteAll,
    #[command(description = "delete specified album (add album ID after `delete` command).")]
    Delete(i64),
}
#[derive(Debug, Deserialize, Clone)]
struct Config {
    teloxide_token: String,
    data_folder: String,
    max_user_folder_size: u32,
    result_folder: String,
    log_path: String,
    restrict_access: bool,
    allowed_users: Vec<u64>
}

fn load_config(file: &str) -> Config {
    let contents = match fs::read_to_string(file) {
        Ok(_a) => _a,
        Err(_) => panic!("Could not read file '{}'", file),
    };

    let config: Config = match toml::from_str(&contents) {
        Ok(_b) => _b,
        Err(_) => panic!("Unable to load data from '{}'", file),
    };

    return config;
}

async fn help(bot: Bot, msg: Message) -> HandlerResult {
    bot.send_message(msg.chat.id, Command::descriptions().to_string()).await?;
    Ok(())
}

async fn showalbums(bot: Bot, msg: Message, config: &Config) -> HandlerResult {
    let user_id = msg.chat.id.0 as u64;
    let mut albums: Option<Vec<String>> = None;
    match agb::get_album_descriptions(user_id, &config.data_folder).await {
        Ok(a) => { albums = Some(a); }
        Err(err) => {
            error!("showalbums(): user #{}: {}", user_id, err);
        }
    }

    if let Some(albums) = albums {
        bot.send_message(msg.chat.id, format!("Available albums:\n\n{}", albums.join("\n"))).await?;
    }
    else {
        bot.send_message(msg.chat.id, format!("❗ No albums found!")).await?;
    }

    Ok(())
}

async fn generateall(bot: Bot, msg: Message, config: &Config) -> HandlerResult {
    let mut counter: Option<u64> = None;
    let mut zip_file: Option<PathBuf> = None;

    // Assume that user ID is the same as chat ID
    let user_id = msg.chat.id.0 as u64;

    // Generate all albums
    match agb::generate_albums(0, user_id, &config.data_folder, &config.result_folder).await {
        Ok(c) => { 
            counter = Some(c.0);
            zip_file = Some(c.1);
        }
        Err(err) => {
            error!("generateall(): user #{}: {}", user_id, err);
        }
    }

    if let Some(counter) = counter {
        bot.send_message(msg.chat.id, format!("✅ Successfully generated {} albums.", counter)).await?;
        bot.send_dice(msg.chat.id).await?;
        let input_file = InputFile::file(&zip_file.unwrap());
        bot.send_document(msg.chat.id, input_file).await?;
        info!("Sent an archive with all albums to user #{}", user_id);
        utils::delete_contents_of_folder(&config.result_folder).await?;
    }
    else {
        bot.send_message(msg.chat.id, format!("❗ Error generating albums!")).await?;
    }

    Ok(())
}

async fn generate(bot: Bot, msg: Message, config: &Config, album_id: i64) -> HandlerResult {
    let mut counter: Option<u64> = None;
    let mut zip_file: Option<PathBuf> = None;
    let mut error_string = String::new();

    // Assume that user ID is the same as chat ID
    let user_id = msg.chat.id.0 as u64;

    // Generate single album
    match agb::generate_albums(album_id, user_id, &config.data_folder, &config.result_folder).await {
        Ok(c) => {
            counter = Some(c.0);
            zip_file = Some(c.1);
        }
        Err(err) => {
            error!("generate(): user #{}: {}", user_id, err);
            error_string = err.to_string();
        }
    }

    if let Some(_) = counter {
        bot.send_message(msg.chat.id, format!("✅ Successfully generated album #{}.", album_id)).await?;
        bot.send_dice(msg.chat.id).await?;
        let input_file = InputFile::file(&zip_file.unwrap());
        bot.send_document(msg.chat.id, input_file).await?;
        info!("Sent an archive with album #{} to user #{}", album_id, user_id);
        utils::delete_contents_of_folder(&config.result_folder).await?;
    }
    else {
        if error_string == "Album not found!" {
            bot.send_message(msg.chat.id, format!("❌ {}", error_string)).await?;
        }
        else {
            bot.send_message(msg.chat.id, format!("❌ Error generating album #{}!", album_id)).await?;
        }
    }

    Ok(())
}

async fn deleteall(bot: Bot, msg: Message, config: &Config) -> HandlerResult {
    let user_id = msg.chat.id.0 as u64;
    let mut ok_string: Option<String> = None;
    let mut error_string = String::new();

    match agb::delete_user_folders(user_id, &config.data_folder).await {
        Ok(res) => {
            ok_string = Some(res);
        }
        Err(err) => {
            error!("deleteall(): user #{}: {}", user_id, err);
            error_string = err.to_string();
        }
    }

    if let Some(message) = ok_string {
        bot.send_message(msg.chat.id, format!("✅ {}", message)).await?;
    }
    else {
        if error_string == "No data found!" {
            bot.send_message(msg.chat.id, format!("❗ {}", error_string)).await?;
        }
        else {
            bot.send_message(msg.chat.id, format!("❌ Error deleting data. Please contact bot owners!")).await?;
        }
    }


    Ok(())
}

async fn delete(bot: Bot, msg: Message, config: &Config, album_id: i64) -> HandlerResult {
    let user_id = msg.chat.id.0 as u64;
    let mut ok_string: Option<String> = None;

    match agb::delete_user_album(album_id, user_id, &config.data_folder).await {
        Ok(res) => {
            ok_string = Some(res);
        }
        Err(err) => {
            error!("delete(): user #{}: {}", user_id, err);
        }
    }

    if let Some(message) = ok_string {
        bot.send_message(msg.chat.id, format!("✅ {}", message)).await?;
    }
    else {
        bot.send_message(msg.chat.id, format!("❌ Error deleting album. Please check album ID and/or contact bot owners!")).await?;
    }

    Ok(())
}

async fn reply_not_authorized(bot: Bot, msg: Message) -> HandlerResult {
    bot.send_message(msg.chat.id, "❗ You are not authorized to use this bot.").await?;
    Ok(())
}

async fn reply(bot: Bot, msg: Message, config: &Config) -> HandlerResult {
    if msg.text() == Some("/start") ||
    msg.text() == Some("/generate") ||
    msg.text() == Some("/delete") {
        return Ok(())
    }

    bot.send_dice(msg.chat.id).await?;

    let chat_id = msg.chat.id.clone();
    let mut ok_string: Option<&str> = None;
    let mut error_string = String::new();

    match agb::add_new_post(bot.clone(), msg, &config.data_folder, config.max_user_folder_size).await {
        Ok(_) => {
            ok_string = Some("Message added to archive.");
        }
        Err(err) => {
            error!("reply(): user #{}: {}", chat_id, err);
            error_string = err.to_string();
        }
    }

    if let Some(message) = ok_string {
        bot.send_message(chat_id, format!("✅ {}", message)).await?;
    }
    else {
        if error_string == "Post already exists!" ||
        error_string == "User folder has exceeded the size limit!" {
            bot.send_message(chat_id, format!("❗ {}", error_string)).await?;
        }
        else {
            bot.send_message(chat_id, format!("❌ Error adding message! Please contact bot owners!")).await?;
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() {
    // Read the config file
    let config = load_config("config.toml");

    let _log2 = log2::open(&config.log_path)
    .module(false)
    .level("info")
    .start();

    info!("Starting bot...");

    let bot = Bot::new(&config.teloxide_token);

    let command_handler = teloxide::filter_command::<Command, _>()
        .branch(dptree::case![Command::Help].endpoint(help))
        .branch(dptree::case![Command::ShowAlbums].endpoint(|bot, msg, config: Config| async move {
            showalbums(bot, msg, &config).await
        }))
        .branch(dptree::case![Command::GenerateAll].endpoint(|bot, msg, config: Config| async move {
            generateall(bot, msg, &config).await
        }))
        .branch(dptree::case![Command::Generate(album_id)].endpoint(|bot, msg, album_id, config: Config| async move {
            generate(bot, msg, &config, album_id).await
        }))
        .branch(dptree::case![Command::DeleteAll].endpoint(|bot, msg, config: Config| async move {
            deleteall(bot, msg, &config).await
        }))
        .branch(dptree::case![Command::Delete(album_id)].endpoint(|bot, msg, album_id, config: Config| async move {
            delete(bot, msg, &config, album_id).await
        }));
   
    let handler = Update::filter_message()
        .branch(dptree::filter(|msg: Message, config: Config| {
                config.restrict_access && !config.allowed_users.contains(&(msg.chat.id.0 as u64))
            })
            .endpoint(reply_not_authorized))
        .branch(command_handler)
        .branch(dptree::endpoint(|bot, msg, config: Config| async move {
            reply(bot, msg, &config).await
        }));

    Dispatcher::builder(bot, handler)
        .dependencies(dptree::deps![config])
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;

    info!("Stopping bot...");

}
