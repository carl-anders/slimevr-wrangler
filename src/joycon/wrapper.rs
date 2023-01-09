use std::{env, sync::mpsc};

use crate::settings;

#[cfg(target_os = "linux")]
use super::linux_integration;
use super::{
    communication::ServerStatus, spawn_thread, test_integration::test_controllers, Communication,
    Status,
};

pub struct Wrapper {
    status_rx: mpsc::Receiver<Vec<Status>>,
    server_rx: mpsc::Receiver<ServerStatus>,
}
impl Wrapper {
    pub fn new(settings: settings::Handler) -> Self {
        let (status_tx, status_rx) = mpsc::channel();
        let (server_tx, server_rx) = mpsc::channel();
        let (tx, rx) = mpsc::channel();
        let settings_clone = settings.clone();
        std::thread::spawn(move || {
            Communication::start(rx, status_tx, server_tx, settings);
        });

        let tx_clone = tx.clone();
        if env::args().any(|a| &a == "test") {
            std::thread::spawn(move || test_controllers(tx_clone));
        }

        // evdev integration
        #[cfg(target_os = "linux")]
        {
            let tx = tx.clone();
            let settings_clone = settings_clone.clone();
            std::thread::spawn(move || linux_integration::spawn_thread(tx, settings_clone));
        }
        std::thread::spawn(move || spawn_thread(tx, settings_clone));
        
        Self {
            status_rx,
            server_rx,
        }
    }
    pub fn poll_status(&self) -> Option<Vec<Status>> {
        self.status_rx.try_iter().last()
    }
    pub fn poll_server(&self) -> Option<ServerStatus> {
        self.server_rx.try_iter().last()
    }
}
