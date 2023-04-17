use std::collections::HashMap;
use std::io;
use std::path::PathBuf;

use crate::utils::systable::SystemTable;

#[derive(Debug)]
pub struct OpsState {
    systable: SystemTable,
    max_bandwidth: u32,

    pub sampling_rates: Vec<u32>,

    pub bands: HashMap<String, Vec<u16>>,
    // TODO: chooser plugin
}

impl OpsState {
    pub fn init(
        systable: &PathBuf,
        max_bandwidth: u32,
        sampling_rates: Vec<u32>,
        plugin: &str,
        args: &HashMap<&str, &str>,
    ) -> io::Result<Self> {
        let table = SystemTable::load(&systable)?;

        // TODO: try to init plugin

        let mut state = Self {
            systable: table,
            max_bandwidth: 0,
            sampling_rates,
            bands: HashMap::new(),
        };
        if !state.set_max_bandwidth(max_bandwidth) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("Invalid max_bandwidth: {}", max_bandwidth),
            ));
        }

        Ok(state)
    }

    pub fn max_bandwidth(&self) -> u32 {
        self.max_bandwidth
    }

    pub fn set_max_bandwidth(&mut self, max_bandwidth: u32) -> bool {
        let bands = self.systable.bands(max_bandwidth);
        if bands.len() == 0 {
            return false;
        }

        self.max_bandwidth = max_bandwidth;
        self.bands = bands;

        true
    }
}
