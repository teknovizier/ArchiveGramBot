ArchiveGramBot
===========================

A bot designed to store Telegram posts containing attached images and videos and generate HTML albums upon request.

Known limitations:
-------
* Only MP4 video format is supported
* If a Telegram post contains multiple videos or photos, each video or photo is saved separately

Setting up
-------

* Create a new bot using [@Botfather](https://t.me/botfather) to get a token
* Rename `config-sample.json` to `config.json`
* Set the values of `teloxide_token`, `data_folder` and `result_folder`
* You can restrict access to the bot for specific Telegram users by setting `restrict_access` to `true` and specifying user Telegram IDs in `allowed_users`

Usage
-------

* Run the app with `cargo run`
