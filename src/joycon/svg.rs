use iced::{svg::Handle, Svg};
use joycon_rs::joycon::JoyConDeviceType;
use std::collections::hash_map::Entry::{Occupied, Vacant};
use std::collections::HashMap;

static LEFT: &str = include_str!("../../assets/joycon-left.svg");
static RIGHT: &str = include_str!("../../assets/joycon-right.svg");

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct JoyconDesign {
    pub colour: [u8; 3],
    pub design_type: JoyConDeviceType,
}
impl JoyconDesign {
    pub fn hex(&self) -> String {
        format!(
            "{:02x}{:02x}{:02x}",
            self.colour[0], self.colour[1], self.colour[2]
        )
    }
}
pub struct JoyconSvg {
    map: HashMap<JoyconDesign, Svg>,
}
impl Default for JoyconSvg {
    fn default() -> JoyconSvg {
        JoyconSvg::new()
    }
}
impl JoyconSvg {
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }
    pub fn get(&mut self, design: JoyconDesign) -> &Svg {
        match self.map.entry(design.clone()) {
            Occupied(entry) => entry.into_mut(),
            Vacant(entry) => {
                let svg_code = match design.design_type {
                    JoyConDeviceType::JoyConL => LEFT.replace("3fa9f5", &design.hex()),
                    JoyConDeviceType::JoyConR | JoyConDeviceType::ProCon => {
                        RIGHT.replace("ff1d25", &design.hex())
                    }
                };

                entry.insert(Svg::new(Handle::from_memory(svg_code)))
            }
        }
    }
}
