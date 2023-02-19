use crate::config::FrequencyBandMap;
use serde_json::Value;
use std::collections::HashMap;

mod single;

pub trait ChooserPlugin {
    /// Invoked to calculate next band to listen to
    fn choose(&mut self) -> Result<&Vec<u32>, String>;

    /// Invoked when a new HFDL frame is received. Returns boolean indicating whether listening bands should change
    fn on_recv_frame(&mut self, frame: &Value) -> bool;

    /// Invoked during listening timeout threshold. Returns boolean indicating whether listening bands should change
    fn on_timeout(&mut self) -> bool;
}

pub fn get<'a, 'b>(
    name: &'a str,
    bands: &'b FrequencyBandMap,
    props: &'b HashMap<&str, &str>,
) -> Option<Box<dyn ChooserPlugin + 'b>> {
    match name {
        single::NAME => Some(Box::new(single::SingleChooserPlugin::new(bands, props))),
        _ => None,
    }
}
