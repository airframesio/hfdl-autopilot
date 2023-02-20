use crate::config::{Config, FrequencyBandMap};
use crate::hfdl::Frame;
use actix_web::web::Data;
use chrono::offset;
use chrono::{DateTime, Utc};
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

#[derive(Debug, Serialize)]
pub struct GroundStationStat {
    pub name: String,
    pub to_msgs: u64,
    pub from_msgs: u64,
    pub last_heard: Option<DateTime<Utc>>,
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

pub fn gs_stats_from_config(config: &Config) -> GroundStationStats {
    let stats = GroundStationStats::new();
    for (_, gs_info) in &config.info.stations {
        stats.insert(
            gs_info.id,
            GroundStationStat {
                name: gs_info.name.clone(),
                to_msgs: 0,
                from_msgs: 0,
                last_heard: None,
            },
        );
    }

    stats
}

pub type PositionReportsByFlightMap = DashMap<String, PositionReports>;

#[derive(Debug, Serialize)]
pub struct PositionReport {
    pub position: Vec<f64>,
    pub propagation: Vec<Vec<f64>>,
}

#[derive(Debug)]
pub struct PositionReports {
    pub last_heard: Instant,
    pub positions: Vec<PositionReport>,
}

impl ser::Serialize for PositionReports {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ser::Serializer,
    {
        let mut state = serializer.serialize_struct("PositionReports", 2)?;
        state.serialize_field("positions", &self.positions)?;
        state.serialize_field("age_in_secs", &self.last_heard.elapsed().as_secs())?;
        state.end()
    }
}

pub struct SharedState {
    bands: FrequencyBandMap,
    spdu_timeout: u64,
    ac_timeout: u64,

    pub gs_info: Data<GroundStationMap>,
    pub gs_stats: Data<GroundStationStats>,
    pub flight_posrpt: Data<PositionReportsByFlightMap>,
    pub freq_stats: Data<FrequencyStats>,
}

impl SharedState {
    pub fn new(config: &Config) -> Self {
        SharedState {
            bands: config.info.bands.clone(),
            spdu_timeout: config.spdu_timeout,
            ac_timeout: config.ac_timeout,

            gs_info: Data::new(gs_info_from_config(config)),
            gs_stats: Data::new(gs_stats_from_config(&config)),
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

    pub fn clean_up(&mut self) {
        let stale_flights: Vec<String> = self
            .flight_posrpt
            .iter()
            .filter(|x| x.value().last_heard.elapsed().as_secs() >= self.ac_timeout)
            .map(|x| x.key().to_string())
            .collect();

        info!("CLEAN UP: Removing stale flights => {:?}", stale_flights);

        for stale_flight in stale_flights.iter() {
            self.flight_posrpt.remove(stale_flight);
        }
    }

    pub fn update(&mut self, msg: &Value) {
        let frame: Frame = match serde_json::from_value(msg.clone()) {
            Ok(val) => val,
            Err(e) => {
                error!("Bad JSON deserialization: not a HFDL Frame: {}", e);
                return;
            }
        };

        {
            *self.freq_stats.entry(frame.hfdl.freq).or_insert(0) += 1;
        }

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

                if let Some(mut entry) = self.gs_info.get_mut(&info.gs.id) {
                    entry.active_bands = bands;
                    entry.last_heard = Some(Instant::now());
                }
            }

            if let Some(mut entry) = self.gs_stats.get_mut(&spdu.src.id) {
                entry.from_msgs += 1;
                entry.last_heard = Some(offset::Utc::now());
            }

            info!(
                " SPDU[{}]({:.1}) {:>4}  {:>13} -> {:<13}  Update Active Freqs",
                frame.hfdl.frequency(),
                frame.hfdl.sig_level,
                frame.hfdl.bit_rate,
                spdu.source(),
                "Broadcast"
            );
        } else if let Some(ref lpdu) = frame.hfdl.lpdu {
            if lpdu.src.entity_name.is_some() {
                if let Some(mut entry) = self.gs_stats.get_mut(&lpdu.src.id) {
                    entry.from_msgs += 1;
                    entry.last_heard = Some(offset::Utc::now());
                }
            }

            if lpdu.dst.entity_name.is_some() {
                if let Some(mut entry) = self.gs_stats.get_mut(&lpdu.dst.id) {
                    entry.to_msgs += 1;
                    entry.last_heard = Some(offset::Utc::now());
                }
            }

            if let Some(ref hfnpdu) = lpdu.hfnpdu {
                if let Some(ref acars) = hfnpdu.acars {
                    info!(
                        "ACARS[{}]({:.1}) {:>4}  {:>13} -> {:<13}  {:<7} {:<2} {:1} {:1}",
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
                        "HFNPD[{}]({:.1}) {:>4}  {:>13} -> {:<13}  {}",
                        frame.hfdl.frequency(),
                        frame.hfdl.sig_level,
                        frame.hfdl.bit_rate,
                        lpdu.source(),
                        lpdu.destination(),
                        hfnpdu.msg_type()
                    );

                    let mut propagation: Vec<Vec<f64>> = vec![];

                    if let Some(ref freq_data) = hfnpdu.freq_data {
                        for info in freq_data {
                            let heard_bands: Vec<u32> = info
                                .heard_on_freqs
                                .iter()
                                .map(|x| self.freq_to_band(x.freq).unwrap_or(0))
                                .collect();
                            if heard_bands.iter().position(|&x| x == 0).is_some() {
                                error!(
                                    "ERROR => found frequency in {:?} that does not match band!",
                                    info.heard_on_freqs
                                );
                                error!("         Most likely data consistency issue, make sure systable.json has proper bandwidth settings!");
                                return;
                            } else if heard_bands.len() > 0 {
                                if let Some(gs) = self.gs_info.get(&info.gs.id) {
                                    propagation.push(gs.position.clone());
                                }

                                if let Some(mut entry) = self.gs_info.get_mut(&info.gs.id) {
                                    if entry.active_bands.len() == 0
                                        || (entry.last_heard.is_some()
                                            && entry.last_heard.unwrap().elapsed().as_secs()
                                                >= self.spdu_timeout)
                                    {
                                        // NOTE: heard-from data is not very reliable and should not be used unless SPDU timed out or isn't populated
                                        entry.active_bands = heard_bands;
                                        entry.last_heard = Some(Instant::now());
                                    }
                                }
                            }
                        }
                    }

                    if hfnpdu.flight_id.is_some() && hfnpdu.pos.is_some() {
                        let pos = hfnpdu.pos.as_ref().unwrap();
                        let report = PositionReport {
                            position: vec![pos.lon, pos.lat],
                            propagation,
                        };

                        if let Some(mut entry) = self
                            .flight_posrpt
                            .get_mut(hfnpdu.flight_id.as_ref().unwrap())
                        {
                            entry.positions.push(report);
                            entry.last_heard = Instant::now();
                        } else {
                            self.flight_posrpt.insert(
                                hfnpdu.flight_id.as_ref().unwrap().clone(),
                                PositionReports {
                                    last_heard: Instant::now(),
                                    positions: vec![report],
                                },
                            );
                        }
                    }
                }
            } else {
                info!(
                    " LPDU[{}]({:.1}) {:>4}  {:>13} -> {:<13}  {}",
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
