use std::{
    collections::HashMap,
    fs,
    fs::File,
    io::BufReader,
    path::PathBuf,
    sync::{Arc, RwLock},
    time::Instant,
};

use directories::ProjectDirs;
use serde::{Deserialize, Serialize};

fn file_name() -> Option<PathBuf> {
    ProjectDirs::from("", "", "SlimeVR Wrangler").map(|pd| pd.config_dir().join("config.json"))
}
#[derive(Serialize, Deserialize, Default, Clone)]
pub struct Joycon {
    pub rotation: i32,
}
#[derive(Serialize, Deserialize, Default, Clone)]
pub struct WranglerSettings {
    pub address: String,
    pub joycon: HashMap<String, Joycon>,
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
                joycon: HashMap::new(),
            })
    }
}

#[derive(Clone)]
pub struct Handler {
    pub local: WranglerSettings,
    remote: Arc<RwLock<WranglerSettings>>,
    updated: Instant,
}
impl Handler {
    pub fn reload(&mut self) {
        if Instant::now().duration_since(self.updated).as_millis() > 250 {
            self.local = self.remote.read().unwrap().clone();
            self.updated = Instant::now();
        }
    }
    pub fn save(&mut self) {
        *self.remote.write().unwrap() = self.local.clone();
        self.local.save();
    }
}
impl Default for Handler {
    fn default() -> Self {
        let settings = WranglerSettings::new();
        Handler {
            local: settings.clone(),
            remote: Arc::new(RwLock::new(settings)),
            updated: Instant::now(),
        }
    }
}
