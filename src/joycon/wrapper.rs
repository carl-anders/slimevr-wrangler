use std::sync::mpsc;

use super::{main_thread, spawn_thread, JoyconStatus};

fn startup(address: String) -> mpsc::Receiver<Vec<JoyconStatus>> {
    let (out_tx, out_rx) = mpsc::channel();
    let (tx, rx) = mpsc::channel();
    let _ = std::thread::spawn(move || main_thread(rx, out_tx, address));
    std::thread::spawn(move || spawn_thread(tx));
    out_rx
}

pub struct JoyconIntegration {
    rx: mpsc::Receiver<Vec<JoyconStatus>>,
}
impl JoyconIntegration {
    pub fn new(address: String) -> Self {
        Self {
            rx: startup(address),
        }
    }
    pub fn poll(&self) -> Option<Vec<JoyconStatus>> {
        self.rx.try_iter().last()
    }
}
