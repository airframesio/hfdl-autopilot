use crate::args::Args;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::fs;
use std::path::PathBuf;

pub type GroundStationMap = HashMap<String, GroundStation>;
pub type FrequencyBandMap = HashMap<u32, Vec<u32>>;

#[derive(Serialize, Deserialize, Debug)]
pub struct GroundStation {
    pub id: u8,
    pub name: String,
    pub lat: f64,
    pub lon: f64,
    pub assigned: Vec<u32>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct HFDLInfo {
    pub stations: GroundStationMap,
    pub bands: FrequencyBandMap,
    pub raw: String,
}

#[derive(Debug)]
pub struct Config {
    pub bin: PathBuf,
    pub timeout: u32,
    pub spdu_timeout: u64,
    pub ac_timeout: u64,
    pub end_session_wait: u64,
    pub additional_args: Vec<String>,

    pub swarm: bool,
    pub host: String,
    pub port: u16,

    pub max_bad_child_reads: u32,

    pub info: HFDLInfo,
}

impl Config {
    fn parse_systable(path: &PathBuf) -> Result<HFDLInfo, String> {
        let contents = match fs::read_to_string(path) {
            Ok(contents) => contents,
            Err(e) => return Err(format!("Unable to read dumphfdl system table: {}", e)),
        };

        serde_json::from_str(&contents)
            .map_err(|e| format!("Unable to deserialize dumphfdl system table: {}", e))
    }

    pub fn from_args(args: &Args) -> Result<Config, String> {
        if !args.bin.exists() || !args.bin.is_file() {
            return Err(format!(
                "dumphfdl binary path does not exist or is not a file: {:?}",
                args.bin
            ));
        }
        if !args.sys_table.exists() || !args.sys_table.is_file() {
            return Err(format!(
                "dumphfdl system table definition does not exist or is not a file: {:?}",
                args.sys_table
            ));
        }

        let info = Config::parse_systable(&args.sys_table)?;

        Ok(Config {
            bin: args.bin.to_owned(),
            timeout: args.timeout,
            spdu_timeout: args.spdu_timeout,
            ac_timeout: args.ac_timeout,
            end_session_wait: args.end_session_wait,
            additional_args: args.additional_args.to_owned(),

            swarm: args.swarm,
            host: args.host.to_owned(),
            port: args.port,

            max_bad_child_reads: 1,

            info,
        })
    }
}

impl fmt::Display for Config {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Cfg[to={{m:{}s a:{}s s:{}s}} sw={} srv={}:{} bin={:?} args={:?}]",
            self.timeout,
            self.ac_timeout,
            self.spdu_timeout,
            if self.swarm { 1 } else { 0 },
            self.host,
            self.port,
            self.bin,
            self.additional_args
        )
    }
}
