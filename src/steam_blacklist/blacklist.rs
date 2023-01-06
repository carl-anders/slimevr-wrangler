use std::{thread, time::Duration};

use super::{Blacklist, BlacklistError, Device};

#[derive(Debug, Clone, Default)]
pub struct BlacklistResult {
    pub info: String,
    pub fix_button: bool,
}
impl BlacklistResult {
    pub fn visible(&self) -> bool {
        !self.info.is_empty()
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
    let list = match Blacklist::read() {
        Ok(l) => l,
        Err(e) => {
            match e {
                BlacklistError::IO(_) | BlacklistError::Parse(_) => {
                    println!("[INFO] Steam config - Could not open steam config file to check for controller blacklist.");
                }
                BlacklistError::Invalid => {
                    println!("[WARNING] Steam config - File invalid.");
                }
                _ => {}
            }
            return BlacklistResult::default();
        }
    };
    let all = [Device::Joycon, Device::SwitchPro];
    match all.iter().filter(|d| list.has(**d)).count() {
        0 => {
            println!("[INFO] Steam config - Blacklist does not contain either Pro controllers or all types of Joycon's.");
            BlacklistResult::fix("Your steam config does not contain a controller blacklist. This will interfere with this app.")
        }
        1 => {
            println!(
                "[INFO] Steam config - Blacklist not fully populated (Joycon's + Pro controller)."
            );
            BlacklistResult::fix("Your steam config blacklist does not contain all types of controllers supported by this app.")
        }
        _ => {
            println!("[INFO] Steam config - Controller blacklist correctly set.");
            BlacklistResult::default()
        }
    }
}

pub async fn check_blacklist() -> BlacklistResult {
    async_std::task::spawn_blocking(inner_check).await
}
fn inner_update() -> BlacklistResult {
    let mut list = match Blacklist::read() {
        Ok(l) => l,
        Err(_) => {
            return BlacklistResult::info("Couldn't update steam controller blacklist.");
        }
    };
    list.add_all();
    match list.save() {
        Ok(_) => {
            BlacklistResult::info("Steam controller blacklist updated. Please restart computer (or at least Steam and this app).")
        },
        Err(e) => {
            match e {
                BlacklistError::Parse(_) | BlacklistError::Invalid => {
                    println!("[INFO] Steam config - Could not open or parse config file to check for controller blacklist.");
                },
                BlacklistError::Regex => {
                    println!("[ERROR] Steam config - Could not parse blacklist with regex.");
                },
                BlacklistError::Update => {
                    println!("[ERROR] Steam config - Could not save config file.");
                },
                BlacklistError::IO(e) => {
                    println!("[ERROR] Could not read/write config file. Full Error:\n{e:?}");
                },
            }
            BlacklistResult::info("Couldn't update steam controller blacklist. More info in console.")
        },
    }
}

pub async fn update_blacklist() -> BlacklistResult {
    async_std::task::spawn_blocking(|| {
        thread::sleep(Duration::from_millis(500)); // Add delay so fixing message can be seen
        inner_update()
    })
    .await
}
