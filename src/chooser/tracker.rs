use log::*;
use std::collections::HashMap;
use std::time::Instant;

use rand::seq::SliceRandom;
use serde::Deserialize;
use serde_json::Value;

use crate::chooser::ChooserPlugin;
use crate::config::FrequencyBandMap;

pub const NAME: &'static str = "tracker";
pub const MAX_VISITED_ENTRIES: usize = 6;

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
struct Frequency {
    id: u8,
    freq: f64,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
struct Entity {
    id: u8,

    #[serde(alias = "type")]
    entity_type: String,

    #[serde(alias = "name")]
    entity_name: Option<String>,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
struct GroundStation {
    gs: Entity,
    utc_sync: bool,
    freqs: Vec<Frequency>,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
struct LPDU {
    err: bool,
    src: Entity,
    dst: Entity,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
struct SPDU {
    err: bool,
    src: Entity,
    gs_status: Vec<GroundStation>,
}

#[derive(Deserialize, Debug)]
struct HFDL {
    freq: u32,
    spdu: Option<SPDU>,
    lpdu: Option<LPDU>,
}

#[derive(Deserialize, Debug)]
struct MessageFrame {
    hfdl: HFDL,
}

pub struct TrackerChooserPlugin {
    recently_visited: Vec<u32>,

    target: Option<String>,
    target_bands: Vec<u32>,
    target_bands_last_updated: Option<Instant>,

    current_band: Option<u32>,
    next_band: Option<u32>,

    last_heard_timeout: u64,
    gs_last_heard: Option<Instant>,
}

impl TrackerChooserPlugin {
    pub fn new() -> Self {
        TrackerChooserPlugin {
            recently_visited: vec![],
            gs_last_heard: None,
            target: None,
            target_bands: vec![],
            target_bands_last_updated: None,
            current_band: None,
            next_band: None,
            last_heard_timeout: 0,
        }
    }

    fn determine_next_band(&mut self) {
        let mut rng = rand::thread_rng();

        if !self.target_bands.is_empty() {
            if self.target_bands_last_updated.is_some()
                && self.target_bands_last_updated.unwrap().elapsed().as_secs()
                    < self.last_heard_timeout * 2
            {
                info!("Recent SPDU containing target GS still fresh...");

                let mut candidates: Vec<u32> = self
                    .target_bands
                    .clone()
                    .into_iter()
                    .filter(|b| b != &self.current_band.unwrap())
                    .collect();
                if candidates.len() > 0 {
                    candidates.shuffle(&mut rng);
                    self.next_band = Some(candidates[0]);
                    info!("Selecting next band: {}", candidates[0]);
                } else {
                    info!("No new bands discovered, ignored.");
                }
            } else {
                info!("Recent SPDU is stale.");
                self.target_bands_last_updated = None;
            }
        }
    }
}

impl ChooserPlugin for TrackerChooserPlugin {
    fn choose<'a, 'b>(
        &mut self,
        bands: &'a FrequencyBandMap,
        props: &'b HashMap<&str, &str>,
    ) -> Result<&'a Vec<u32>, String> {
        if self.target.is_none() {
            self.target = props.get("target").map(|s| s.to_string());
            if self.target.is_none() {
                return Err("No target specified".to_string());
            }
        }

        if self.last_heard_timeout == 0 {
            self.last_heard_timeout = match props.get("timeout").unwrap_or(&"600").parse() {
                Ok(secs) => secs,
                Err(e) => return Err(format!("'timeout' is not a valid positive number: {}", e)),
            };
        }

        let next_band: u32;

        if self.next_band.is_none() {
            let mut rng = rand::thread_rng();
            let mut band_keys: Vec<&u32> = bands.keys().into_iter().collect();
            band_keys.shuffle(&mut rng);

            while !self.recently_visited.is_empty()
                && self
                    .recently_visited
                    .iter()
                    .position(|&b| b == *band_keys[0])
                    .is_some()
            {
                band_keys.remove(0);
            }

            next_band = *band_keys[0];
        } else {
            next_band = self.next_band.unwrap();
            self.next_band = None;
        }

        if self.recently_visited.len() == MAX_VISITED_ENTRIES {
            self.recently_visited.remove(0);
        }
        self.recently_visited.push(next_band);
        self.current_band = Some(next_band);

        bands
            .get(&next_band)
            .ok_or(format!("Invalid band: {}", next_band))
    }

    fn on_update(&mut self, frame: &Value) -> bool {
        let target = self.target.as_ref().unwrap();

        let msg: MessageFrame = match serde_json::from_value(frame.clone()) {
            Ok(m) => m,
            Err(e) => {
                error!("Failed to coerce frame into MessageFrame: {}", e);
                return false;
            }
        };

        let freq = msg.hfdl.freq / 1000;
        let mut spdu_contains_target = false;

        if msg.hfdl.spdu.is_some() {
            let spdu = msg.hfdl.spdu.unwrap();

            let src_entity = spdu.src.entity_name;
            if src_entity.is_some() && src_entity.unwrap().starts_with(target) {
                info!("Received SPDU on {} from target GS: {}", freq, target);
                self.gs_last_heard = Some(Instant::now());
            }

            for station in spdu.gs_status.iter() {
                if station.gs.entity_name.is_some()
                    && station.gs.entity_name.as_ref().unwrap().starts_with(target)
                {
                    self.target_bands.clear();
                    self.target_bands.extend_from_slice(
                        station
                            .freqs
                            .iter()
                            .map(|f| (f.freq / 1000.0) as u32)
                            .collect::<Vec<u32>>()
                            .as_slice(),
                    );
                    self.target_bands_last_updated = Some(Instant::now());

                    info!(
                        "Found SPDU containing target GS freqs: {:?}",
                        self.target_bands
                    );
                    spdu_contains_target = true;
                    break;
                }
            }
        } else if msg.hfdl.lpdu.is_some() {
            let lpdu = msg.hfdl.lpdu.unwrap();

            let src_entity = lpdu.src.entity_name;
            if src_entity.is_some() && src_entity.unwrap().starts_with(target) {
                info!("Received LPDU on {} from target GS: {}", freq, target);
                self.gs_last_heard = Some(Instant::now());
            }

            let dst_entity = lpdu.dst.entity_name;
            if dst_entity.is_some() && dst_entity.unwrap().starts_with(target) {
                info!("Received LPDU on {} to target GS: {}", freq, target);
                self.gs_last_heard = Some(Instant::now());
            }
        }

        if let Some(timer) = self.gs_last_heard {
            if timer.elapsed().as_secs() > self.last_heard_timeout {
                self.gs_last_heard = None;

                info!(
                    "Been too long (>{}s) since last message heard to/from target GS {}",
                    self.last_heard_timeout,
                    self.target.as_ref().unwrap()
                );

                self.determine_next_band();
                return true;
            }
        } else if spdu_contains_target {
            info!("Switching bands to bands heard from SPDU containing target");
            self.determine_next_band();
            return true;
        }

        false
    }

    fn on_timeout(&mut self) -> bool {
        self.determine_next_band();

        true
    }
}
