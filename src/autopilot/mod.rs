use std::io;
use std::sync::RwLock;

use actix_web::web::Data;
use actix_web::{App, HttpServer};
use serde_json::Value;
use url::Url;

use crate::utils;

pub mod settings;
pub mod state;

#[derive(Debug)]
pub struct Autopilot {
    swarm_url: Option<Url>,

    pub settings: Data<RwLock<settings::Settings>>,
    pub state: Data<RwLock<state::OpsState>>,
}

impl Autopilot {
    pub fn new(settings: settings::Settings, state: state::OpsState) -> Self {
        Autopilot {
            swarm_url: None,

            settings: Data::new(RwLock::new(settings)),
            state: Data::new(RwLock::new(state)),
        }
    }

    pub fn enable_swarm(&mut self, url: &Url) {
        self.swarm_url = Some(url.to_owned());
    }

    pub fn enable_api_server(
        &mut self,
        host: &str,
        port: u16,
        token: &str,
        disable_cors: bool,
        disable_api_control: bool,
    ) {
        self.swarm_url = None;

        // TODO: init actix
    }

    pub fn choose_listening_freqs(&mut self) -> io::Result<(Vec<u16>, u32)> {
        let state = self.state.read().unwrap();

        // TODO: reset "forced change" flag

        // TODO: use current state's chooser plugin to figure out next
        let freqs = vec![8912, 8991];
        let sampling_rate = utils::get_sampling_rate(&freqs, &state.sampling_rates)?;

        Ok((freqs, sampling_rate))
    }

    pub fn should_run(&self) -> bool {
        true
    }

    pub fn on_timeout(&mut self) -> bool {
        true
    }

    pub fn on_frame(&mut self, frame: &Value) -> bool {
        // TODO: should_change()
        false
    }

    pub fn should_change(&mut self) -> bool {
        false
    }

    pub fn cleanup(&mut self) {}
}
