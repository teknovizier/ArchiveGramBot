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
    result_folder: String,
    log_path: String,
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
        bot.send_message(msg.chat.id, format!("No albums found!")).await?;
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
        bot.send_message(msg.chat.id, format!("Successfully generated {} albums.", counter)).await?;
        bot.send_dice(msg.chat.id).await?;
        let input_file = InputFile::file(&zip_file.unwrap());
        bot.send_document(msg.chat.id, input_file).await?;
        info!("Sent an archive with all albums to user #{}", user_id);
        utils::delete_contents_of_folder(&config.result_folder).await?;
    }
    else {
        bot.send_message(msg.chat.id, format!("Error generating albums!")).await?;
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
        bot.send_message(msg.chat.id, format!("Successfully generated album #{}.", album_id)).await?;
        bot.send_dice(msg.chat.id).await?;
        let input_file = InputFile::file(&zip_file.unwrap());
        bot.send_document(msg.chat.id, input_file).await?;
        info!("Sent an archive with album #{} to user #{}", album_id, user_id);
        utils::delete_contents_of_folder(&config.result_folder).await?;
    }
    else {
        if error_string == "Album not found!" {
            bot.send_message(msg.chat.id, error_string).await?;
        }
        else {
            bot.send_message(msg.chat.id, format!("Error generating album #{}!", album_id)).await?;
        }
    }

    Ok(())
}

async fn deleteall(bot: Bot, msg: Message, config: &Config) -> HandlerResult {
    let user_id = msg.chat.id.0 as u64;
    let mut ok_string: Option<&str> = None;

    match agb::delete_user_folders(user_id, &config.data_folder).await {
        Ok(_) => {
            ok_string = Some("All data deleted.");
        }
        Err(err) => {
            error!("deleteall(): user #{}: {}", user_id, err);
        }
    }

    if let Some(_) = ok_string {
        bot.send_message(msg.chat.id, format!("All data deleted.")).await?;
    }
    else {
        bot.send_message(msg.chat.id, format!("Error deleting data. Please contact bot owners!")).await?;
    }

    Ok(())
}

async fn delete(bot: Bot, msg: Message, config: &Config, album_id: i64) -> HandlerResult {
    let user_id = msg.chat.id.0 as u64;
    let mut ok_string: Option<&str> = None;

    match agb::delete_user_album(album_id, user_id, &config.data_folder).await {
        Ok(_) => {
            ok_string = Some("All data deleted.");
        }
        Err(err) => {
            error!("delete(): user #{}: {}", user_id, err);
        }
    }

    if let Some(_) = ok_string {
        bot.send_message(msg.chat.id, format!("Album #{} deleted.", album_id)).await?;
    }
    else {
        bot.send_message(msg.chat.id, format!("Error deleting album. Please check album ID and/or contact bot owners!")).await?;
    }

    Ok(())
}

async fn reply(bot: Bot, msg: Message, config: &Config) -> HandlerResult {
    if msg.text() == Some("/start") {
        return Ok(())
    }

    bot.send_dice(msg.chat.id).await?;

    let chat_id = msg.chat.id.clone();
    let mut ok_string: Option<&str> = None;
    let mut error_string = String::new();

    match agb::add_new_post(bot.clone(), msg, &config.data_folder).await {
        Ok(_) => {
            ok_string = Some("Message added to archive.");
        }
        Err(err) => {
            error!("reply(): user #{}: {}", chat_id, err);
            error_string = err.to_string();
        }
    }

    if let Some(ok) = ok_string {
        bot.send_message(chat_id, format!("{}", ok)).await?;
    }
    else {
        if error_string == "Messages with multiple photos/videos aren't supported yet!" ||
        error_string == "Post already exists!" {
            bot.send_message(chat_id, error_string).await?;
        }
        else {
            bot.send_message(chat_id, format!("Error adding message! Please contact bot owners!")).await?;
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
        .branch(dptree::case![Command::ShowAlbums].endpoint({
            let config = config.clone();
            move |bot, msg| {
                let config = config.clone();
                async move { showalbums(bot, msg, &config).await }
            }
        }))
        .branch(dptree::case![Command::GenerateAll].endpoint({
            let config = config.clone();
            move |bot, msg| {
                let config = config.clone();
                async move { generateall(bot, msg, &config).await }
            }
        }))
        .branch(dptree::case![Command::Generate(album_id)].endpoint({
            let config = config.clone();
            move |bot, msg, album_id| {
                let config = config.clone();
                async move { generate(bot, msg, &config, album_id).await }
            }
        }))
        .branch(dptree::case![Command::DeleteAll].endpoint({
            let config = config.clone();
            move |bot, msg| {
                let config = config.clone();
                async move { deleteall(bot, msg, &config).await }
            }
        }))
        .branch(dptree::case![Command::Delete(album_id)].endpoint({
            let config = config.clone();
            move |bot, msg, album_id| {
                let config = config.clone();
                async move { delete(bot, msg, &config, album_id).await }
            }
        }));

    let handler = Update::filter_message()
        .branch(command_handler)
        .branch(
            dptree::filter(|msg: Message| {
                msg.chat.id != ChatId(0)
            })
            .endpoint({
                let config = config.clone();
                move |bot, msg| {
                    let config = config.clone();
                    async move { reply(bot, msg, &config).await }
                }
            }));

    Dispatcher::builder(bot, handler)
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;

    info!("Stopping bot...");

}
