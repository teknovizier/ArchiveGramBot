ArchiveGramBot
===========================

A bot designed to store Telegram posts containing attached images and videos and generate HTML albums upon request.

Known limitations
-------

* Only MP4 video format is supported
* Maximum media file size to be processed by bot is 5 MB for a photo and 20 MB for a video as limited by [Telegram Bot API](https://core.telegram.org/bots/api)
* Maximum album archive size to be sent by bot automatically is 20 MB

Setting up
-------

* Create a new bot using [@Botfather](https://t.me/botfather) to get a token
* Rename `config-sample.toml` to `config.toml`
* Set the values of `teloxide_token`, `data_folder` and `result_folder`
* You can restrict access to the bot for specific Telegram users by setting `restrict_access` to `true` and specifying user Telegram IDs in `allowed_users`

Usage
-------

* Run the app with `cargo run`
