use log::*;
use rand::Rng;
use serde_json::Value;
use std::collections::HashMap;

use crate::chooser::ChooserPlugin;
use crate::config::FrequencyBandMap;

pub const NAME: &'static str = "rotate";

const MAX_MEMORY_ENTRIES: usize = 8;

pub struct RotateChooserPlugin {
    recently_used: Vec<usize>,
    band_idx: Option<usize>,
}

impl RotateChooserPlugin {
    pub fn new() -> Self {
        RotateChooserPlugin {
            band_idx: None,
            recently_used: vec![],
        }
    }
}

impl ChooserPlugin for RotateChooserPlugin {
    fn choose<'a, 'b>(
        &mut self,
        bands: &'a FrequencyBandMap,
        props: &'b HashMap<&str, &str>,
    ) -> Result<&'a Vec<u32>, String> {
        let mut band_keys: Vec<&u32> = bands.keys().into_iter().collect();
        band_keys.sort_unstable();

        let switcher = *props.get("type").unwrap_or(&"inc");

        if self.band_idx.is_none() {
            let start: u32 = match props.get("start").unwrap_or(&"13").parse() {
                Ok(start) => start,
                Err(e) => {
                    return Err(format!(
                        "'start' key contains an invalid positive number: {}",
                        e
                    ))
                }
            };

            self.band_idx = band_keys.iter().position(|&b| b == &start);
            if self.band_idx.is_none() {
                return Err(format!("'start' key value ({}) is not a valid band", start));
            }

            self.recently_used.push(self.band_idx.unwrap());
        } else if switcher.eq("dec") {
            info!("[dec]    current band_idx = {:?}", self.band_idx);

            if self.band_idx.unwrap() == 0 {
                self.band_idx = Some(band_keys.len() - 1);
            } else {
                self.band_idx = Some(self.band_idx.unwrap() - 1);
            }

            info!("[dec]    next band_idx = {:?}", self.band_idx);
        } else if switcher.eq("random") {
            let mut new_idx = self.band_idx.unwrap();
            info!(
                "[random] current band_idx = {:?}, recently_used = {:?}",
                self.band_idx, self.recently_used
            );

            while self
                .recently_used
                .iter()
                .position(|&b| b == new_idx)
                .is_some()
            {
                new_idx = rand::thread_rng().gen_range(0..(band_keys.len() - 1))
            }

            if self.recently_used.len() == MAX_MEMORY_ENTRIES {
                self.recently_used.remove(0);
            }

            self.recently_used.push(new_idx);
            self.band_idx = Some(new_idx);

            info!("[random] next band_idx = {:?}", self.band_idx);
        } else {
            info!("[inc]    current band_idx = {:?}", self.band_idx);

            if self.band_idx.unwrap() + 1 >= band_keys.len() {
                self.band_idx = Some(0);
            } else {
                self.band_idx = Some(self.band_idx.unwrap() + 1);
            }

            info!("[inc]    next band_idx = {:?}", self.band_idx);
        }

        let band = band_keys[self.band_idx.unwrap()];
        bands.get(&band).ok_or(format!("Invalid band: {}", band))
    }

    fn on_update(&mut self, _frame: &Value) -> bool {
        false
    }

    fn on_timeout(&mut self) -> bool {
        true
    }
}
