use crate::config::{Config, FrequencyBandMap};
use crate::hfdl::Frame;
use actix_web::web::Data;
use dashmap::DashMap;
use log::*;
use serde::ser;
use serde::ser::SerializeStruct;
use serde::Serialize;
use serde_json::Value;
use std::time::Instant;

pub type FrequencyStats = DashMap<u32, u32>;
pub type GroundStationStats = DashMap<u8, GroundStationStat>;
pub type GroundStationMap = DashMap<u8, GroundStationInfo>;

#[derive(Debug)]
pub struct GroundStationStat {
    pub name: String,
    pub to_msgs: u64,
    pub from_msgs: u64,
    // TOOD:  last_heard_ts
}

#[derive(Debug)]
pub struct GroundStationInfo {
    pub name: String,
    pub position: Vec<f64>,
    pub active_bands: Vec<u32>,

    pub last_heard: Option<Instant>,
}

impl ser::Serialize for GroundStationInfo {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ser::Serializer,
    {
        let mut state = serializer.serialize_struct("GroundStationInfo", 4)?;
        state.serialize_field("name", &self.name)?;
        state.serialize_field("position", &self.position)?;
        state.serialize_field("active_bands", &self.active_bands)?;
        state.serialize_field(
            "age_in_secs",
            &self.last_heard.map(|i| i.elapsed().as_secs()),
        )?;
        state.end()
    }
}

pub fn gs_info_from_config(config: &Config) -> GroundStationMap {
    let info = GroundStationMap::new();
    for (_, gs_info) in &config.info.stations {
        info.insert(
            gs_info.id,
            GroundStationInfo {
                name: gs_info.name.clone(),
                position: vec![gs_info.lon, gs_info.lat],
                active_bands: vec![],
                last_heard: None,
            },
        );
    }

    info
}

pub type PositionReportsByFlightMap = DashMap<String, PositionReports>;

#[derive(Debug, Serialize)]
pub struct PositionReport {
    pub position: Vec<f64>,
    pub propagation: Vec<Vec<f64>>,
}

#[derive(Debug)]
pub struct PositionReports {
    pub last_heard: Option<Instant>,

    pub positions: Vec<PositionReport>,
}

impl ser::Serialize for PositionReports {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ser::Serializer,
    {
        let mut state = serializer.serialize_struct("PositionReports", 2)?;
        state.serialize_field("positions", &self.positions)?;
        state.serialize_field(
            "age_in_secs",
            &self.last_heard.map(|i| i.elapsed().as_secs()),
        )?;
        state.end()
    }
}

pub struct SharedState {
    bands: FrequencyBandMap,

    pub gs_info: Data<GroundStationMap>,
    pub gs_stats: Data<GroundStationStats>,
    pub flight_posrpt: Data<PositionReportsByFlightMap>,
    pub freq_stats: Data<FrequencyStats>,
}

impl SharedState {
    pub fn new(config: &Config) -> Self {
        SharedState {
            bands: config.info.bands.clone(),

            gs_info: Data::new(gs_info_from_config(config)),
            gs_stats: Data::new(GroundStationStats::new()),
            flight_posrpt: Data::new(PositionReportsByFlightMap::new()),
            freq_stats: Data::new(FrequencyStats::new()),
        }
    }

    pub fn freq_to_band(&self, freq: f64) -> Option<u32> {
        for (band, freqs) in &self.bands {
            if freqs.iter().position(|&x| x == (freq as u32)).is_some() {
                return Some(*band);
            }
        }

        None
    }

    pub fn update(&mut self, msg: &Value) {
        let frame: Frame = match serde_json::from_value(msg.clone()) {
            Ok(val) => val,
            Err(e) => {
                error!("Bad JSON deserialization: not a HFDL Frame: {}", e);
                return;
            }
        };

        if let Some(ref spdu) = frame.hfdl.spdu {
            for info in &spdu.gs_status {
                let bands: Vec<u32> = info
                    .freqs
                    .iter()
                    .map(|x| self.freq_to_band(x.freq).unwrap_or(0))
                    .collect();

                if bands.iter().position(|&x| x == 0).is_some() {
                    error!(
                        "ERROR => found frequency in {:?} that does not match band!",
                        info.freqs
                    );
                    error!("         Most likely data consistency issue, make sure systable.json has proper bandwidth settings!");
                    return;
                }

                match self.gs_info.get_mut(&info.gs.id) {
                    Some(mut entry) => {
                        entry.active_bands = bands;
                        entry.last_heard = Some(Instant::now());
                    }
                    None => {}
                }
            }

            info!(
                " SPDU[{}]({:.1}) {:>4}bps  {} -> ALL  Update Active Freqs",
                frame.hfdl.frequency(),
                frame.hfdl.sig_level,
                frame.hfdl.bit_rate,
                spdu.source()
            );
            info!(
                "    GS => {:?}",
                spdu.gs_status
                    .iter()
                    .map(|x| x.gs.entity_name.as_ref().unwrap())
                    .collect::<Vec<&String>>()
            );
        } else if let Some(ref lpdu) = frame.hfdl.lpdu {
            if let Some(ref hfnpdu) = lpdu.hfnpdu {
                if let Some(ref acars) = hfnpdu.acars {
                    info!(
                        "ACARS[{}]({:.1}) {:>4}bps  {} -> {}  {:<7} {:<2} {:1} {:1}",
                        frame.hfdl.frequency(),
                        frame.hfdl.sig_level,
                        frame.hfdl.bit_rate,
                        lpdu.source(),
                        lpdu.destination(),
                        acars.flight.as_ref().unwrap_or(&" ".to_string()),
                        acars.label,
                        acars.blk_id,
                        acars.ack
                    );
                } else {
                    info!(
                        "  HFN[{}]({:.1}) {:>4}bps  {} -> {}  {}",
                        frame.hfdl.frequency(),
                        frame.hfdl.sig_level,
                        frame.hfdl.bit_rate,
                        lpdu.source(),
                        lpdu.destination(),
                        hfnpdu.msg_type()
                    );
                }
            } else {
                info!(
                    " LPDU[{}]({:.1}) {:>4}bps  {} -> {}  {}",
                    frame.hfdl.frequency(),
                    frame.hfdl.sig_level,
                    frame.hfdl.bit_rate,
                    lpdu.source(),
                    lpdu.destination(),
                    lpdu.msg_type()
                );
            }
        }
    }
}
