use iced::{svg::Handle, Svg};
use std::collections::hash_map::Entry::{Occupied, Vacant};
use std::collections::HashMap;

static LEFT: &str = include_str!("../../assets/joycon-left.svg");
static RIGHT: &str = include_str!("../../assets/joycon-right.svg");

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum JoyconDesignType {
    Left,
    Right
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct JoyconDesign {
    pub color: String,
    pub design_type: JoyconDesignType,
}
pub struct JoyconSvg {
    map: HashMap<JoyconDesign, Svg>,
}
impl Default for JoyconSvg {
    fn default() -> Self {
        Self::new()
    }
}
impl JoyconSvg {
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }
    pub fn get(&mut self, design: &JoyconDesign) -> &Svg {
        match self.map.entry(design.clone()) {
            Occupied(entry) => entry.into_mut(),
            Vacant(entry) => {
                let svg_code = match design.design_type {
                    JoyconDesignType::Left => LEFT.replace("#3fa9f5", &design.color),
                    JoyconDesignType::Right => RIGHT.replace("#ff1d25", &design.color)
                };

                entry.insert(Svg::new(Handle::from_memory(svg_code)))
            }
        }
    }
}
