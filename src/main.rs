use log2::*;
use serde::Deserialize;
use std::fs;
use teloxide::{prelude::*, utils::command::BotCommands};
use archivegrambot as agb;

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
    Generate(u64),
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

async fn answer(bot: Bot, msg: Message, cmd: Command, config: &Config) -> ResponseResult<()> {
    let mut counter: Option<u64> = None;
    match cmd {
        Command::Help => bot.send_message(msg.chat.id, Command::descriptions().to_string()).await?,
        Command::ShowAlbums => {
            let user_id = msg.chat.id.0 as u64;
            let mut albums: Option<Vec<String>> = None;
            match agb::get_album_descriptions(user_id, &config.data_folder).await {
                Ok(a) => { albums = Some(a); }
                Err(_) => {}
            }

            if let Some(albums) = albums {
                bot.send_message(msg.chat.id, format!("Available albums:\n\n{}", albums.join("\n"))).await?
            }
            else {
                bot.send_message(msg.chat.id, format!("No albums found!")).await?
            }

        }
        Command::GenerateAll => {
            // Assume that user ID is the same as chat ID
            let user_id = msg.chat.id.0 as u64;

            // Generate all albums
            match agb::generate_albums(0, user_id, &config.data_folder, &config.result_folder).await {
                Ok(c) => { counter = Some(c); }
                Err(_) => {}
            }

            if let Some(counter) = counter {
                bot.send_message(msg.chat.id, format!("Successfully generated {} albums.", counter)).await?
            }
            else {
                bot.send_message(msg.chat.id, format!("Error generating albums!")).await?
            }
        }
        Command::Generate(album_id) => {
            // Assume that user ID is the same as chat ID
            let user_id = msg.chat.id.0 as u64;

            // Generate single album
            match agb::generate_albums(album_id, user_id, &config.data_folder, &config.result_folder).await {
                Ok(c) => { counter = Some(c); }
                Err(_) => {}
            }

            if let Some(_) = counter {
                bot.send_message(msg.chat.id, format!("Successfully generated album #{}.", album_id)).await?
            }
            else {
                bot.send_message(msg.chat.id, format!("Error generating album #{}!", album_id)).await?
            }
        }
    };

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
    Command::repl(bot, move |bot, msg, cmd| {
        let config = config.clone();
        async move {
            answer(bot, msg, cmd, &config).await
        }
    }).await;

    info!("Stopping bot...");
}
