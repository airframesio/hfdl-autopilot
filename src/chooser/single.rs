use crate::{chooser::ChooserPlugin, config::FrequencyBandMap};
use std::collections::HashMap;

pub const NAME: &'static str = "single";

pub struct SingleChooserPlugin<'a> {
    bands: &'a FrequencyBandMap,
    props: &'a HashMap<&'a str, &'a str>,
}

impl<'a> SingleChooserPlugin<'a> {
    pub fn new(bands: &'a FrequencyBandMap, props: &'a HashMap<&'a str, &'a str>) -> Self {
        SingleChooserPlugin { bands, props }
    }
}

impl<'a> ChooserPlugin for SingleChooserPlugin<'a> {
    fn choose(&mut self) -> Result<&'a Vec<u32>, String> {
        if !self.props.contains_key("band") {
            return Err("Missing 'band' key in props".to_string());
        }

        let band: u32 = match self.props.get("band").unwrap().parse() {
            Ok(band) => band,
            Err(e) => {
                return Err(format!(
                    "'band' key contains an invalid positive number: {}",
                    e
                ))
            }
        };

        self.bands
            .get(&band)
            .ok_or(format!("Invalid band: {}", band))
    }

    fn on_recv_frame(&mut self, _frame: &serde_json::Value) -> bool {
        false
    }

    fn on_timeout(&mut self) -> bool {
        false
    }
}
