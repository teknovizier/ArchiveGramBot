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

Usage
-------

* Run the app with `cargo run`
