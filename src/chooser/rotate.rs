use crate::{chooser::ChooserPlugin, config::FrequencyBandMap};
use log::*;
use rand::rngs::ThreadRng;
use rand::Rng;
use std::collections::HashMap;

pub const NAME: &'static str = "rotate";

pub struct RotateChooserPlugin<'a> {
    bands: &'a FrequencyBandMap,

    rng: ThreadRng,

    switcher: &'a str,
    ignore_last: usize,

    band_keys: Vec<&'a u32>,
    recently_used: Vec<usize>,
    init_band_idx: usize,

    band_idx: Option<usize>,
}

impl<'a> RotateChooserPlugin<'a> {
    pub fn new(
        bands: &'a FrequencyBandMap,
        props: &'a HashMap<&'a str, &'a str>,
    ) -> Result<Self, String> {
        let mut band_keys: Vec<&u32> = bands.keys().into_iter().collect();
        band_keys.sort_unstable();

        let start_band: u32 = props.get("start").unwrap_or(&"13").parse().unwrap_or(13);
        let init_band_idx = band_keys
            .iter()
            .position(|&&x| x == start_band)
            .unwrap_or(0);

        Ok(RotateChooserPlugin {
            bands,

            rng: rand::thread_rng(),

            switcher: *props.get("type").unwrap_or(&"inc"),
            ignore_last: props
                .get("ignore_last")
                .unwrap_or(&"DEFAULT")
                .parse()
                .unwrap_or(8),

            band_keys,
            recently_used: vec![],
            init_band_idx,

            band_idx: None,
        })
    }
}

impl<'a> ChooserPlugin for RotateChooserPlugin<'a> {
    fn choose(&mut self) -> Result<&'a Vec<u32>, String> {
        if self.band_idx.is_none() {
            self.recently_used.push(self.init_band_idx);
            self.band_idx = Some(self.init_band_idx);
        } else if self.switcher.eq("random") {
            info!(
                "[rotate](random) band_idx = {:?}, recently_used = {:?}, ignore_last = {}",
                self.band_idx, self.recently_used, self.ignore_last
            );

            let mut new_idx = self.band_idx.unwrap();
            while self.recently_used.iter().any(|&x| x == new_idx) {
                new_idx = self.rng.gen_range(0..(self.band_keys.len() - 1));
            }

            if self.recently_used.len() >= self.ignore_last {
                self.recently_used.remove(0);
            }
            self.recently_used.push(new_idx);
            self.band_idx = Some(new_idx);
        } else if self.switcher.eq("dec") {
            let band_idx = self.band_idx.unwrap();
            self.band_idx = Some(if band_idx == 0 {
                self.band_keys.len() - 1
            } else {
                band_idx - 1
            });
        } else {
            let next_idx = self.band_idx.unwrap() + 1;
            self.band_idx = Some(if next_idx >= self.band_keys.len() {
                0
            } else {
                next_idx
            });
        }

        let band = self.band_keys[self.band_idx.unwrap()];
        self.bands
            .get(band)
            .ok_or(format!("Invalid band: {}", band))
    }

    fn on_recv_frame(&mut self, _frame: &serde_json::Value) -> bool {
        false
    }

    fn on_timeout(&mut self) -> bool {
        true
    }
}
