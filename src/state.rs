use crate::config::Config;
use log::*;
use serde_json::Value;

pub struct SharedState {
    // TODO: active GS bands from SPDU and heard-froms
}

impl SharedState {
    pub fn new(config: &Config) -> Self {
        SharedState {}
    }

    pub fn update(&mut self, frame: &Value) {}
}
