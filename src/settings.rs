use std::{
    collections::HashMap, fs, fs::File, io::BufReader, net::SocketAddr, path::PathBuf, sync::Arc,
};

use arc_swap::{ArcSwap, Guard};
use directories::ProjectDirs;
use rand::Rng;
use serde::{Deserialize, Serialize};

fn file_name() -> Option<PathBuf> {
    ProjectDirs::from("", "", "SlimeVR Wrangler").map(|pd| pd.config_dir().join("config.json"))
}
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Joycon {
    #[serde(default)]
    pub rotation: i32,
    #[serde(default = "return_f64_one")]
    pub gyro_scale_factor: f64,
    #[serde(default)]
    pub keep_id: u8,
}
fn return_f64_one() -> f64 {
    1.0
}

impl Default for Joycon {
    fn default() -> Self {
        Joycon {
            rotation: 0,
            gyro_scale_factor: 1.0,
            keep_id: 0,
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct WranglerSettings {
    pub address: String,
    #[serde(default)]
    pub joycon: HashMap<String, Joycon>,
    #[serde(default = "return_true")]
    pub send_reset: bool,
    #[serde(default = "return_mac")]
    pub emulated_mac: [u8; 6],
    #[serde(default = "return_false")]
    pub keep_ids: bool,
}

fn return_true() -> bool {
    true
}
fn return_false() -> bool {
    false
}
fn return_mac() -> [u8; 6] {
    let mut r = rand::rng();
    [0x00, 0x0F, r.random(), r.random(), r.random(), r.random()]
}

const DEFAULT_ADDR: &str = "127.0.0.1:6969";

impl WranglerSettings {
    pub fn save(&self) {
        let file = file_name().unwrap();
        if !file.exists() {
            fs::create_dir_all(file.parent().unwrap()).unwrap();
        }
        File::create(file)
            .ok()
            .and_then(|file| serde_json::to_writer_pretty(file, self).ok());
    }
    pub fn load_and_save() -> Self {
        let settings = file_name()
            .and_then(|path| File::open(path).ok())
            .and_then(|file| serde_json::from_reader(BufReader::new(file)).ok())
            .unwrap_or_else(|| Self {
                address: DEFAULT_ADDR.into(),
                joycon: HashMap::new(),
                send_reset: true,
                emulated_mac: return_mac(),
                keep_ids: false,
            });
        settings.save();
        settings
    }
    pub fn joycon_rotation_add(&mut self, serial_number: String, degrees: i32) {
        let entry = self.joycon.entry(serial_number).or_default();
        entry.rotation = (entry.rotation + degrees).rem_euclid(360);
    }
    pub fn joycon_rotation_get(&self, serial_number: &str) -> i32 {
        self.joycon.get(serial_number).map_or(0, |j| j.rotation)
    }
    pub fn joycon_scale_set(&mut self, serial_number: String, scale: f64) {
        let entry = self.joycon.entry(serial_number).or_default();
        entry.gyro_scale_factor = scale;
    }
    pub fn joycon_scale_get(&self, serial_number: &str) -> f64 {
        self.joycon
            .get(serial_number)
            .map_or(1.0, |j| j.gyro_scale_factor)
    }
    fn joycon_keep_id_set_new(&mut self, serial_number: String) {
        let max = self.joycon.values().map(|j| j.keep_id).max();
        let entry = self.joycon.entry(serial_number).or_default();
        entry.keep_id = max.unwrap_or_default().saturating_add(1);
        if entry.keep_id == u8::MAX {
            println!("\x1b[0;31m[ERROR]\x1b[0m TOO MANY JOYCONS SAVED! THIS WILL BREAK THINGS!");
            println!(" YOU NEED TO DISABLE THE \"Save mounting location on server\" SETTING!!!");
        }
    }
    pub fn get_socket_address(&self) -> SocketAddr {
        self.address
            .parse::<SocketAddr>()
            .unwrap_or_else(|_| DEFAULT_ADDR.parse().unwrap())
    }
}
impl Default for WranglerSettings {
    fn default() -> Self {
        WranglerSettings::load_and_save()
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
    pub fn joycon_keep_id(&self, serial_number: String) -> u8 {
        let keep_id = self
            .load()
            .joycon
            .get(&serial_number)
            .map_or(0, |j| j.keep_id);
        if keep_id != 0 {
            return keep_id;
        }
        self.change(|ws| ws.joycon_keep_id_set_new(serial_number.clone()));
        self.load()
            .joycon
            .get(&serial_number)
            .map_or(0, |j| j.keep_id)
    }
}
