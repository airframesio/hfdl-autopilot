use crate::chooser::ChooserPlugin;
use crate::config::FrequencyBandMap;
use crate::utils::{get_band, parse_time};
use log::*;
use std::collections::HashMap;

pub const NAME: &'static str = "schedule";

pub struct ScheduleChooserPlugin<'a> {
    bands: &'a FrequencyBandMap,

    triggers: Vec<(u8, u8, u32)>,
    current_band: u32,
}

impl<'a> ScheduleChooserPlugin<'a> {
    pub fn new(
        bands: &'a FrequencyBandMap,
        props: &'a HashMap<&'a str, &'a str>,
    ) -> Result<Self, String> {
        let mut triggers: Vec<(u8, u8, u32)> = vec![];

        for (k, v) in props.iter() {
            let time: Vec<&str> = k.split(":").collect();
            if time.len() != 2 {
                continue;
            }

            let (h, m) = match parse_time(&time) {
                Some(val) => val,
                None => continue,
            };

            let band: u32 = v.parse().unwrap_or(0);
            if bands.contains_key(&band) && !triggers.iter().any(|x| x.0 == h && x.1 == m) {
                triggers.push((h, m, band))
            }
        }

        if triggers.is_empty() {
            return Err(format!("No valid band switch triggers found: {:?}", props));
        }

        triggers.sort_unstable();

        info!(
            "schedule: bands sked = {:?}",
            triggers
                .iter()
                .map(|x| format!("{:02}:{:02} => {}", x.0, x.1, x.2))
                .collect::<Vec<String>>()
        );

        Ok(ScheduleChooserPlugin {
            bands,
            triggers,
            current_band: 0,
        })
    }
}

impl<'a> ChooserPlugin for ScheduleChooserPlugin<'a> {
    fn choose(&mut self) -> Result<&'a Vec<u32>, String> {
        let band = match get_band(&self.triggers) {
            Some(val) => val,
            None => {
                return Err(format!(
                    "schedule: could not find band with triggers => {:?}",
                    self.triggers
                ))
            }
        };

        self.current_band = band;
        self.bands
            .get(&band)
            .ok_or(format!("Invalid band: {}", band))
    }

    fn on_recv_frame(&mut self, _frame: &serde_json::Value) -> bool {
        let band = get_band(&self.triggers).unwrap_or(0);
        band != 0 && self.current_band != band
    }

    fn on_timeout(&mut self) -> bool {
        false
    }
}
