use log2::*;
use teloxide::prelude::*;

mod handlers;
mod operations;
mod utils;

use handlers::Command;
use utils::Config;

#[tokio::main]
async fn main() {
    // Read the config file
    let config = utils::load_config("config.toml");

    let _log2 = log2::open(&config.log_path)
        .module(false)
        .level("info")
        .start();

    info!("Starting bot...");

    let bot = Bot::new(&config.teloxide_token);

    let command_handler = teloxide::filter_command::<Command, _>()
        .branch(dptree::case![Command::Help].endpoint(handlers::help))
        .branch(dptree::case![Command::ShowAlbums].endpoint(
            |bot, msg, config: Config| async move { handlers::showalbums(bot, msg, &config).await },
        ))
        .branch(
            dptree::case![Command::ConsolidateAll].endpoint(
                |bot, msg, config: Config| async move {
                    handlers::consolidateall(bot, msg, &config).await
                },
            ),
        )
        .branch(
            dptree::case![Command::GenerateAll].endpoint(|bot, msg, config: Config| async move {
                handlers::generateall(bot, msg, &config).await
            }),
        )
        .branch(dptree::case![Command::Generate(username)].endpoint(
            |bot, msg, username, config: Config| async move {
                handlers::generate(bot, msg, &config, username).await
            },
        ))
        .branch(
            dptree::case![Command::DeleteAll].endpoint(|bot, msg, config: Config| async move {
                handlers::deleteall(bot, msg, &config).await
            }),
        )
        .branch(dptree::case![Command::Delete(username)].endpoint(
            |bot, msg, username, config: Config| async move {
                handlers::delete(bot, msg, &config, username).await
            },
        ));

    let handler = Update::filter_message()
        .branch(
            dptree::filter(|msg: Message, config: Config| {
                config.restrict_access && !config.allowed_users.contains(&(msg.chat.id.0 as u64))
            })
            .endpoint(handlers::reply_not_authorized),
        )
        .branch(command_handler)
        .branch(dptree::endpoint(|bot, msg, config: Config| async move {
            handlers::reply(bot, msg, &config).await
        }));

    Dispatcher::builder(bot, handler)
        .dependencies(dptree::deps![config])
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;

    info!("Stopping bot...");
}
