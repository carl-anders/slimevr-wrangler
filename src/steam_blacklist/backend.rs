use std::{fs, io, mem::take, path::PathBuf};

use itertools::Itertools;
use keyvalues_parser::Vdf;
use regex::{Captures, Regex};
use thiserror::Error;

fn check_valid(config: &Vdf) -> Result<(), BlacklistError> {
    config
        .value
        .get_obj()
        .and_then(|o| o.get("Software"))
        .map_or(false, |s| !s.is_empty())
        .then_some(())
        .ok_or(BlacklistError::Invalid)
}

fn get_blacklist<'a>(config: &'a Vdf<'a>) -> Option<&'a str> {
    config
        .value
        .get_obj()?
        .get("controller_blacklist")?
        .get(0)?
        .get_str()
}

#[cfg(target_os = "windows")]
fn get_steam_path() -> io::Result<PathBuf> {
    let hklm = winreg::RegKey::predef(winreg::enums::HKEY_LOCAL_MACHINE);
    hklm.open_subkey("SOFTWARE\\Wow6432Node\\Valve\\Steam")
        .or_else(|_| hklm.open_subkey("SOFTWARE\\Valve\\Steam"))?
        .get_value::<String, _>("InstallPath")
        .map(PathBuf::from)
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

fn set_blacklist<'a>(
    raw_text: &str,
    config: &Vdf<'a>,
    new_list: &str,
) -> Result<String, BlacklistError> {
    let output = match get_blacklist(config) {
        Some(old_list) => {
            let re = Regex::new(&format!(
                r#"((?i)"controller_blacklist"\s*)"{}""#,
                regex::escape(old_list)
            ))
            .unwrap();
            if re.find_iter(raw_text).count() != 1 {
                return Err(BlacklistError::Regex);
            }
            re.replace(raw_text, |caps: &Captures| {
                format!(r#"{}"{}""#, &caps[1], new_list)
            })
        }
        None => {
            let re = Regex::new(r"(\}\s*)$").unwrap();
            if re.find_iter(raw_text).count() != 1 {
                return Err(BlacklistError::Regex);
            }
            re.replace(raw_text, |caps: &Captures| {
                format!(
                    "\t\"controller_blacklist\"\t\t\"{}\"\n{}",
                    new_list, &caps[1]
                )
            })
        }
    };
    Ok(output.into())
}

fn verify(config_text: &str, new_list: &str) -> Result<(), BlacklistError> {
    let config = Vdf::parse(config_text)?;
    if let Some(new_list_parsed) = get_blacklist(&config) {
        if new_list_parsed == new_list {
            return Ok(());
        }
    }
    Err(BlacklistError::Update)
}

fn inner_save(new_text: &str) -> Result<(), BlacklistError> {
    let path = get_steam_config_path()?;
    fs::write(path, new_text)?;
    Ok(())
}

#[derive(Debug, Clone, Copy)]
pub enum Device {
    Joycon,
    SwitchPro,
}

impl Device {
    pub fn ids(self) -> Vec<String> {
        match self {
            Device::Joycon => vec![
                "0x057e/0x2006".into(),
                "0x057e/0x2007".into(),
                "0x057e/0x2008".into(),
            ],
            Device::SwitchPro => vec!["0x057e/0x2009".into()],
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct Blacklist {
    devices: Vec<String>,
}

impl Blacklist {
    pub fn add_all(&mut self) {
        self.add(Device::Joycon);
        self.add(Device::SwitchPro);
    }
    pub fn has(&self, device: Device) -> bool {
        device.ids().iter().all(|d| self.devices.contains(d))
    }
    pub fn add(&mut self, device: Device) {
        self.devices = take(&mut self.devices)
            .into_iter()
            .chain(device.ids().into_iter())
            .unique()
            .collect();
    }
    /*pub fn remove(&mut self, device: Device) {
        self.devices.retain(|d| !device.ids().contains(d))
    }*/
    pub fn read() -> Result<Self, BlacklistError> {
        let config_text = read_config()?;
        let config = Vdf::parse(&config_text)?;
        check_valid(&config)?;

        let devices = get_blacklist(&config)
            .map(|l| {
                l.split(',')
                    .map(str::to_lowercase)
                    .filter(|s| !s.is_empty())
                    .collect()
            })
            .unwrap_or_default();
        Ok(Self { devices })
    }
    pub fn save(&self) -> Result<(), BlacklistError> {
        let config_text = read_config()?;
        let config = Vdf::parse(&config_text)?;
        check_valid(&config)?;

        let new_list = self.devices.join(",");
        let new_text = set_blacklist(&config_text, &config, &new_list)?;
        verify(&new_text, &new_list)?;
        inner_save(&new_text)?;
        Ok(())
    }
}

#[derive(Error, Debug)]
pub enum BlacklistError {
    #[error("Read/write error")]
    IO(#[from] io::Error),
    #[error("Parse error")]
    Parse(#[from] keyvalues_parser::error::Error),
    #[error("Invalid config")]
    Invalid,
    #[error("Regex parse error")]
    Regex,
    #[error("Update error")]
    Update,
}
