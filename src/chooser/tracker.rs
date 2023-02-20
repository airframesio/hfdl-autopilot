use crate::chooser::ChooserPlugin;
use crate::config::{Config, FrequencyBandMap};
use crate::hfdl::{Entity, Frame};
use crate::state::GroundStationMap;
use actix_web::web::Data;
use log::*;
use rand::rngs::ThreadRng;
use rand::seq::SliceRandom;
use std::collections::HashMap;
use std::time::Instant;

pub const NAME: &'static str = "tracker";

pub struct TrackerChooserPlugin<'a> {
    bands: &'a FrequencyBandMap,
    props: &'a HashMap<&'a str, &'a str>,
    gs_info: Data<GroundStationMap>,

    rng: ThreadRng,

    target_id: u8,
    spdu_timeout: u64,
    last_heard_timeout: u64,
    last_heard: Option<Instant>,

    current_band: u32,
}

impl<'a> TrackerChooserPlugin<'a> {
    pub fn new(
        config: &'a Config,
        props: &'a HashMap<&'a str, &'a str>,
        gs_info: Data<GroundStationMap>,
    ) -> Result<Self, String> {
        let target_gs = match props.get("target") {
            Some(prefix) => *prefix,
            None => return Err("Missing 'target' property".to_string()),
        };

        let target_id: u8 = match target_gs.parse::<u8>() {
            Ok(id) => {
                if gs_info.get(&id).is_none() {
                    return Err(format!("{} is not a valid ground station ID", id));
                }

                id
            }
            Err(_) => {
                let mut id: Option<u8> = None;
                for item in gs_info.iter() {
                    if item.value().name.starts_with(target_gs) {
                        id = Some(*item.key());
                        break;
                    }
                }

                match id {
                    Some(val) => val,
                    None => {
                        return Err(format!(
                            "'{}' doesn't match any ground station name prefixes",
                            target_gs
                        ))
                    }
                }
            }
        };

        Ok(TrackerChooserPlugin {
            bands: &config.info.bands,
            props,
            gs_info,

            rng: rand::thread_rng(),

            target_id,
            spdu_timeout: config.spdu_timeout,
            last_heard_timeout: props
                .get("last_heard_timeout")
                .unwrap_or(&"DEFAULT")
                .parse()
                .unwrap_or((config.spdu_timeout / 3) as u64),
            last_heard: None,

            current_band: 0,
        })
    }
}

impl<'a> TrackerChooserPlugin<'a> {
    fn frame_involves_target(&self, entity: &Entity) -> bool {
        entity.entity_type.eq_ignore_ascii_case("ground station") && entity.id == self.target_id
    }
}

impl<'a> ChooserPlugin for TrackerChooserPlugin<'a> {
    fn choose(&mut self) -> Result<&'a Vec<u32>, String> {
        let gs = match self.gs_info.get(&self.target_id) {
            Some(val) => val,
            None => return Err(format!("Invalid target GS ID: #{}", self.target_id)),
        };

        let mut bands: Vec<u32> = (if gs.active_bands.len() == 0
            || gs.last_heard.unwrap_or(Instant::now()).elapsed().as_secs() > self.spdu_timeout
        {
            gs.assigned_bands.clone()
        } else {
            gs.active_bands.clone()
        })
        .into_iter()
        .filter(|&x| x != self.current_band)
        .collect();

        if bands.len() == 0 {
            return Err(format!(
                "Candidate bands is empty: last_heard={:?} spdu_timeout={}",
                gs.last_heard, self.spdu_timeout
            ));
        }

        bands.shuffle(&mut self.rng);
        let band = bands[0];

        self.bands
            .get(&band)
            .ok_or(format!("Invalid band: {}", band))
    }

    fn on_recv_frame(&mut self, frame: &serde_json::Value) -> bool {
        let msg: Frame = match serde_json::from_value(frame.clone()) {
            Ok(val) => val,
            Err(e) => {
                error!("Bad JSON decode of frame: {}", e);
                return false;
            }
        };

        if let Some(lpdu) = msg.hfdl.lpdu {
            if self.frame_involves_target(&lpdu.dst) || self.frame_involves_target(&lpdu.src) {
                self.last_heard = Some(Instant::now());
            }
        } else if let Some(spdu) = msg.hfdl.spdu {
            if self.frame_involves_target(&spdu.src) {
                self.last_heard = Some(Instant::now());
            }
        }

        let elapsed_secs = self
            .last_heard
            .unwrap_or(Instant::now())
            .elapsed()
            .as_secs();
        let change_bands = elapsed_secs >= self.last_heard_timeout;
        if change_bands {
            info!(
                "Been {}s (timeouts after {}s) since last target GS #{} frame. Chooser elects to switch bands.",
                elapsed_secs, self.last_heard_timeout, self.target_id
            );
        }

        change_bands
    }

    fn on_timeout(&mut self) -> bool {
        true
    }
}