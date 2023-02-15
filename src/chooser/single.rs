use std::collections::HashMap;

use serde_json::Value;

use crate::chooser::ChooserPlugin;
use crate::config::FrequencyBandMap;

pub const NAME: &'static str = "single";

pub struct SingleChooserPlugin {}

impl SingleChooserPlugin {
    pub fn new() -> Self {
        SingleChooserPlugin {}
    }
}

impl ChooserPlugin for SingleChooserPlugin {
    fn choose<'a, 'b>(
        &mut self,
        bands: &'a FrequencyBandMap,
        props: &'b HashMap<&str, &str>,
    ) -> Result<&'a Vec<u32>, String> {
        if !props.contains_key("band") {
            return Err("Missing 'band' key in props".to_string());
        }

        let band: u32 = match props.get("band").unwrap().parse() {
            Ok(band) => band,
            Err(e) => {
                return Err(format!(
                    "'band' key contains an invalid positive number: {}",
                    e
                ))
            }
        };

        bands.get(&band).ok_or(format!("Invalid band: {}", band))
    }

    fn on_update(&mut self, _frame: &Value) -> bool {
        false
    }

    fn on_timeout(&mut self) -> bool {
        false
    }
}
