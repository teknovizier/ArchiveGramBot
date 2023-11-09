use log2::*;
use serde::Deserialize;
use std::fs;
use archivegrambot::generate_albums;

#[derive(Debug, Deserialize)]
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

pub fn main() {
    // Read the config file
    let config = load_config("config.toml");

    let _log2 = log2::open(&config.log_path)
    .module(false)
    .level("info")
    .start();
    info!("Starting ArchiveGram...");

    // Use dummy user ID for now
    let user_id = 1;
    let _ = match generate_albums(user_id, &config.data_folder, &config.result_folder) {
        Ok(c) => { info!("Successfully generated {} albums.", c)},
        Err(e) => { error!("Error generating albums: {}", e) }
    };

    info!("Stopping ArchiveGram...");
}
