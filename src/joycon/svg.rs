use iced::widget::svg::Handle;
use std::{
    cell::RefCell,
    collections::{
        hash_map::Entry::{Occupied, Vacant},
        HashMap,
    },
};

static LEFT: &str = include_str!("../../assets/joycon-left.svg");
static RIGHT: &str = include_str!("../../assets/joycon-right.svg");

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum JoyconDesignType {
    Left,
    Right,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct JoyconDesign {
    pub color: String,
    pub design_type: JoyconDesignType,
}

fn generate(design: &JoyconDesign, rotation: i32) -> Handle {
    let svg_code = match design.design_type {
        JoyconDesignType::Left => LEFT,
        JoyconDesignType::Right => RIGHT,
    }
    .replace("#3fa9f5", &design.color)
    .replace("rotate(0", &format!("rotate({:}", (rotation + 90) % 360));
    // Rotation is how many degrees clockwise joycons are rotated from their "starting position".
    // Left starts with rail down. Right starts with rail up.
    // The svg's are not consistent with that so needs to be rotated an extra 90 degrees.
    Handle::from_memory(svg_code.as_bytes().to_vec())
}

#[derive(Clone, Debug)]
pub struct Svg {
    map: RefCell<HashMap<(JoyconDesign, i32), Handle>>,
}
impl Svg {
    pub fn new() -> Self {
        Self {
            map: RefCell::new(HashMap::new()),
        }
    }
    pub fn get(&self, design: &JoyconDesign, rotation: i32) -> Handle {
        match self.map.borrow_mut().entry((design.clone(), rotation)) {
            Occupied(entry) => entry.get().clone(),
            Vacant(entry) => entry.insert(generate(design, rotation)).clone(),
        }
    }
}
