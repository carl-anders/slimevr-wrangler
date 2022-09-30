use std::{fs, io, path::PathBuf, thread, time::Duration};

use itertools::Itertools;
use keyvalues_parser::Vdf;
use regex::{Captures, Regex};

fn check_valid(config: &Vdf) -> bool {
    config
        .value
        .get_obj()
        .and_then(|o| o.get("Software"))
        .map(|s| s.len() > 0)
        .unwrap_or(false)
}

fn get_blacklist<'a>(config: &'a Vdf<'a>) -> Option<&'a str> {
    config
        .value
        .get_obj()?
        .get("controller_blacklist")?
        .get(0)?
        .get_str()
}

const IDS: [&'static str; 4] = [
    "0x057e/0x2006",
    "0x057e/0x2007",
    "0x057e/0x2008",
    "0x057e/0x2009",
];

fn check_list(list: &str) -> bool {
    let ids: Vec<_> = list.split(',').collect();
    IDS.iter().all(|i| ids.contains(&i.to_lowercase().as_str()))
}

#[cfg(target_os = "windows")]
fn get_steam_path() -> io::Result<PathBuf> {
    let hklm = winreg::RegKey::predef(winreg::enums::HKEY_LOCAL_MACHINE);
    hklm.open_subkey("SOFTWARE\\Wow6432Node\\Valve\\Steam")
        .or_else(|_| hklm.open_subkey("SOFTWARE\\Valve\\Steam"))?
        .get_value("InstallPath")
        .and_then(|path: String| Ok(PathBuf::from(path)))
}
#[cfg(not(target_os = "windows"))]
fn get_steam_path() -> io::Result<PathBuf> {
    Err(io::Error::from(io::ErrorKind::NotFound))
}
fn get_steam_config_path() -> io::Result<PathBuf> {
    let mut path = get_steam_path()?;
    path.push("config");
    path.push("config.vdf");
    Ok(path)
}

fn read_config() -> io::Result<String> {
    fs::read_to_string(get_steam_config_path()?)
}

#[derive(Debug, Clone, Default)]
pub struct BlacklistResult {
    pub info: String,
    pub fix_button: bool,
}
impl BlacklistResult {
    pub fn visible(&self) -> bool {
        self.info.len() > 0
    }
    pub fn fix<S: Into<String>>(info: S) -> Self {
        Self {
            info: info.into(),
            fix_button: true,
        }
    }
    pub fn info<S: Into<String>>(info: S) -> Self {
        Self {
            info: info.into(),
            fix_button: false,
        }
    }
}

fn inner_check() -> BlacklistResult {
    let config_text = match read_config() {
        Ok(ct) => ct,
        Err(_) => {
            println!("[INFO] Could not open steam config file to check for controller blacklist.");
            return BlacklistResult::default();
        }
    };
    let config = match Vdf::parse(&config_text) {
        Ok(c) => c,
        Err(_) => {
            println!("[WARNING] Steam config not valid.");
            return BlacklistResult::default();
        }
    };
    if !check_valid(&config) {
        println!("[WARNING] Steam config not valid.");
        return BlacklistResult::default();
    }
    match get_blacklist(&config) {
        Some(blacklist) => {
            if check_list(blacklist) {
                println!("[INFO] Steam config - Controller blacklist correctly set.");
                return BlacklistResult::default();
            } else {
                println!("[INFO] Steam config - Blacklist not fully populated (Joycon's + Pro controller).");
                return BlacklistResult::fix("Your steam config blacklist does not contain all types of controllers supported by this app.");
            }
        }
        None => {
            println!("[INFO] Blacklist not correct");
            return BlacklistResult::fix("Your steam config does not contain a controller blacklist. This will interfere with this app.");
        }
    }
}

pub async fn check_blacklist() -> BlacklistResult {
    async_std::task::spawn_blocking(|| inner_check()).await
}

fn add_to_list(list: &str) -> String {
    let lowercase = list.to_lowercase();
    let ids: Vec<_> = lowercase.split(',').filter(|&x| !x.is_empty()).collect();
    ids.iter().chain(IDS.iter()).unique().join(",")
}

fn set_blacklist<'a>(raw_text: &str, config: &Vdf<'a>, new_list: &str) -> Option<String> {
    let output = match get_blacklist(config) {
        Some(old_list) => {
            let re = Regex::new(&format!(
                r#"((?i)"controller_blacklist"\s*)"{}""#,
                regex::escape(old_list)
            ))
            .unwrap();
            if re.find_iter(&raw_text).count() != 1 {
                println!("[ERR] Could not parse blacklist with regex.");
                return None;
            }
            re.replace(&raw_text, |caps: &Captures| {
                format!(r#"{}"{}""#, &caps[1], new_list)
            })
        }
        None => {
            let re = Regex::new(r"(\}\s*)$").unwrap();
            if re.find_iter(&raw_text).count() != 1 {
                println!("[ERR] Could not parse blacklist with regex.");
                return None;
            }
            re.replace(&raw_text, |caps: &Captures| {
                format!(
                    "\t\"controller_blacklist\"\t\t\"{}\"\n{}",
                    new_list, &caps[1]
                )
            })
        }
    };
    Some(output.to_string())
}
fn verify(new_text: &str, new_list: &str) -> bool {
    if let Some(new_config) = Vdf::parse(&new_text).ok() {
        if let Some(new_list_parsed) = get_blacklist(&new_config) {
            return new_list_parsed == new_list;
        }
    }
    println!("[ERR] Could not correctly update config file.");
    false
}
fn save(new_text: &str) -> bool {
    let path = match get_steam_config_path() {
        Ok(p) => p,
        Err(_) => {
            return false;
        }
    };

    match fs::write(path, new_text) {
        Ok(()) => true,
        Err(e) => {
            println!("[ERR] Could not save config file.");
            dbg!(e);
            false
        }
    }
}

fn inner_update() -> BlacklistResult {
    if let Some(text) = read_config().ok() {
        if let Some(config) = Vdf::parse(&text.clone()).ok() {
            if check_valid(&config) {
                let new_list = add_to_list(get_blacklist(&config).unwrap_or(""));
                if let Some(new_text) = set_blacklist(&text, &config, &new_list) {
                    if verify(&new_text, &new_list) {
                        if save(&new_text) {
                            return BlacklistResult::info("Steam controller blacklist updated. Please restart computer (or at least Steam and this app).");
                        }
                    }
                }
            }
        }
    }
    BlacklistResult::info(
        "Couldn't update steam controller blacklist. Console might have more info.",
    )
}

pub async fn update_blacklist() -> BlacklistResult {
    async_std::task::spawn_blocking(|| {
        thread::sleep(Duration::from_millis(500)); // Add delay so fixing message can be seen
        inner_update()
    })
    .await
}
