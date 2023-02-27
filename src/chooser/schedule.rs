use crate::chooser::ChooserPlugin;
use crate::config::FrequencyBandMap;
use chrono::Timelike;
use log::*;
use std::collections::HashMap;

pub const NAME: &'static str = "schedule";

pub struct ScheduleChooserPlugin<'a> {
    bands: &'a FrequencyBandMap,

    triggers: Vec<(u8, u8, u32)>,
    current_band: u32,
}

fn parse_time(raw_time: &Vec<&str>) -> Option<(u8, u8)> {
    let h: u8 = raw_time[0].parse().unwrap_or(255);
    let m: u8 = raw_time[1].parse().unwrap_or(255);

    if h > 23 || m >= 60 {
        return None;
    }

    Some((h, m))
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

    fn get_band(&self) -> Option<u32> {
        let current_time = chrono::offset::Local::now();

        for (h, m, band) in self.triggers.iter() {
            if current_time.hour() >= (*h as u32) && current_time.minute() >= (*m as u32) {
                return Some(*band);
            }
        }

        self.triggers.last().map(|x| x.2)
    }
}

impl<'a> ChooserPlugin for ScheduleChooserPlugin<'a> {
    fn choose(&mut self) -> Result<&'a Vec<u32>, String> {
        let band = match self.get_band() {
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
        let band = self.get_band().unwrap_or(0);
        band != 0 && self.current_band != band
    }

    fn on_timeout(&mut self) -> bool {
        false
    }
}
