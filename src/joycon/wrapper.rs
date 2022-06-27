use std::sync::mpsc;

use crate::settings;

use super::{main_thread, spawn_thread, JoyconStatus};

fn startup(settings: settings::Handler) -> mpsc::Receiver<Vec<JoyconStatus>> {
    let (out_tx, out_rx) = mpsc::channel();
    let (tx, rx) = mpsc::channel();
    let settings_clone = settings.clone();
    let _drop = std::thread::spawn(move || main_thread(rx, out_tx, settings));
    std::thread::spawn(move || spawn_thread(tx, settings_clone));
    out_rx
}

pub struct JoyconIntegration {
    rx: mpsc::Receiver<Vec<JoyconStatus>>,
}
impl JoyconIntegration {
    pub fn new(settings: settings::Handler) -> Self {
        Self {
            rx: startup(settings),
        }
    }
    pub fn poll(&self) -> Option<Vec<JoyconStatus>> {
        self.rx.try_iter().last()
    }
}
