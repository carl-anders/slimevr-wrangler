use iced::widget::svg::Handle;
use std::{
    cell::RefCell,
    collections::{
        hash_map::Entry::{Occupied, Vacant},
        HashMap,
    },
};

static NEEDLE: &str = include_str!("../assets/needle.svg");

fn generate(rotation: i32) -> Handle {
    let svg_code = NEEDLE.replace("rotate(0", &format!("rotate({:}", rotation));
    Handle::from_memory(svg_code.as_bytes().to_vec())
}

#[derive(Clone, Debug)]
pub struct Needle {
    map: RefCell<HashMap<i32, Handle>>,
}
impl Needle {
    pub fn new() -> Self {
        Self {
            map: RefCell::new(HashMap::new()),
        }
    }
    pub fn get(&self, rotation: i32) -> Handle {
        match self.map.borrow_mut().entry(rotation) {
            Occupied(entry) => entry.get().clone(),
            Vacant(entry) => entry.insert(generate(rotation)).clone(),
        }
    }
}
