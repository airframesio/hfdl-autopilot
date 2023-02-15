use crate::config::FrequencyBandMap;
use serde_json::Value;
use std::collections::HashMap;

pub mod rotate;
pub mod single;
pub mod tracker;

pub trait ChooserPlugin {
    fn choose<'a, 'b>(
        &mut self,
        bands: &'a FrequencyBandMap,
        props: &'b HashMap<&str, &str>,
    ) -> Result<&'a Vec<u32>, String>;

    fn on_update(&mut self, frame: &Value) -> bool;
    fn on_timeout(&mut self) -> bool;
}

pub fn get(name: &str) -> Option<Box<dyn ChooserPlugin>> {
    match name {
        rotate::NAME => Some(Box::new(rotate::RotateChooserPlugin::new())),
        single::NAME => Some(Box::new(single::SingleChooserPlugin::new())),
        tracker::NAME => Some(Box::new(tracker::TrackerChooserPlugin::new())),
        _ => None,
    }
}
