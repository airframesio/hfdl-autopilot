use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::{env, fmt, fs};

pub type GroundStationMap = HashMap<String, GroundStation>;
pub type FrequencyBandMap = HashMap<u32, Vec<u32>>;

#[derive(Serialize, Deserialize, Debug)]
pub struct GroundStation {
    id: u32,
    name: String,
    lat: f64,
    lon: f64,
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
    pub driver: String,
    pub output: Option<String>,
    pub timeout: u32,

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

    pub fn from_args(args: &crate::args::Args) -> Result<Config, String> {
        if !args.bin.exists() || !args.bin.is_file() {
            return Err(format!(
                "dumphfdl binary path does not exist or is not a file: {:?}",
                args.bin
            ));
        }
        if !args.sys_table.exists() || !args.bin.is_file() {
            return Err(format!(
                "dumphfdl system table definition does not exist or is not a file: {:?}",
                args.sys_table
            ));
        }

        let soapy_driver = env::var("HFDLAP_SOAPY_DRIVER").map_or_else(
            |_| args.driver.clone(),
            |val| {
                if val.len() > 0 {
                    val
                } else {
                    args.driver.clone()
                }
            },
        );

        let info = Config::parse_systable(&args.sys_table)?;

        Ok(Config {
            bin: args.bin.clone(),
            driver: soapy_driver,
            output: args.output.clone(),
            timeout: args.timeout,
            info,
        })
    }
}

impl fmt::Display for Config {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Config {{ bin={:?}, driver={}, output={:?} timeout={}s }}",
            self.bin, self.driver, self.output, self.timeout
        )
    }
}
