use std::{collections::HashMap, fs, fs::File, io::BufReader, path::PathBuf, sync::Arc};

use arc_swap::{ArcSwap, Guard};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};

fn file_name() -> Option<PathBuf> {
    ProjectDirs::from("", "", "SlimeVR Wrangler").map(|pd| pd.config_dir().join("config.json"))
}
#[derive(Serialize, Deserialize, Clone)]
pub struct Joycon {
    pub rotation: i32,
    pub gyro_scale_factor: f64, //[f64; 3],
}

impl Default for Joycon {
    fn default() -> Self {
        Joycon {
            rotation: 0,
            gyro_scale_factor: 1.0, //[1.0; 3]
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct WranglerSettings {
    pub address: String,
    #[serde(default)]
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
    pub fn load() -> Self {
        file_name()
            .and_then(|path| File::open(path).ok())
            .and_then(|file| serde_json::from_reader(BufReader::new(file)).ok())
            .unwrap_or_else(|| Self {
                address: "127.0.0.1:6969".into(),
                joycon: HashMap::new(),
            })
    }
    pub fn joycon_rotation_add(&mut self, serial_number: String, degrees: i32) {
        let entry = self.joycon.entry(serial_number).or_default();
        entry.rotation = (entry.rotation + degrees).rem_euclid(360);
    }
    pub fn joycon_rotation_get(&self, serial_number: &str) -> i32 {
        self.joycon.get(serial_number).map_or(0, |j| j.rotation)
    }
}
impl Default for WranglerSettings {
    fn default() -> Self {
        WranglerSettings::load()
    }
}

#[derive(Default, Clone)]
pub struct Handler {
    arc: Arc<ArcSwap<WranglerSettings>>,
}
impl Handler {
    pub fn load(&self) -> Guard<Arc<WranglerSettings>> {
        self.arc.load()
    }
    pub fn change<T>(&self, func: T)
    where
        T: FnOnce(&mut WranglerSettings),
    {
        let mut current = (**self.arc.load()).clone();
        func(&mut current);
        current.save();
        self.arc.store(Arc::new(current));
    }
}
