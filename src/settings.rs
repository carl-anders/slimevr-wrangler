use std::{fs, fs::File, io::BufReader, path::PathBuf};

use directories::ProjectDirs;
use serde::{Deserialize, Serialize};

fn file_name() -> Option<PathBuf> {
    ProjectDirs::from("", "", "SlimeVR Wrangler")
        .map(|pd| pd.config_dir().join("config.json"))
}
#[derive(Serialize, Deserialize, Default)]
pub struct WranglerSettings {
    pub address: String,
}
impl WranglerSettings {
    pub fn save(&self) {
        let file = file_name().unwrap();
        if !file.exists() {
            fs::create_dir_all(file.parent().unwrap()).unwrap();
        }
        File::create(file)
            .ok()
            .and_then(|file| serde_json::to_writer(file, self).ok());
    }
    pub fn new() -> Self {
        file_name()
            .and_then(|path| File::open(path).ok())
            .and_then(|file| serde_json::from_reader(BufReader::new(file)).ok())
            .unwrap_or_else(|| Self {
                address: "127.0.0.1:6969".into(),
            })
    }
}
