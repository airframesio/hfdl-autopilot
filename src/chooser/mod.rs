use crate::config::Config;
use crate::state::GroundStationMap;
use actix_web::web::Data;
use serde_json::Value;
use std::collections::HashMap;

mod rotate;
mod single;
mod tracker;

macro_rules! init_plugin {
    ($l:expr) => {
        match $l {
            Ok(plugin) => Box::new(plugin),
            Err(e) => return Err(e),
        }
    };
}

pub trait ChooserPlugin {
    /// Invoked to calculate next band to listen to
    fn choose(&mut self) -> Result<&Vec<u32>, String>;

    /// Invoked when a new HFDL frame is received. Returns boolean indicating whether listening bands should change
    fn on_recv_frame(&mut self, frame: &Value) -> bool;

    /// Invoked during listening timeout threshold. Returns boolean indicating whether listening bands should change
    fn on_timeout(&mut self) -> bool;
}

pub fn get<'a, 'b>(
    name: &'a str,
    config: &'b Config,
    props: &'b HashMap<&str, &str>,
    gs_info: Data<GroundStationMap>,
) -> Result<Box<dyn ChooserPlugin + 'b>, String> {
    let chooser: Box<dyn ChooserPlugin> = match name {
        rotate::NAME => init_plugin!(rotate::RotateChooserPlugin::new(&config.info.bands, props)),
        single::NAME => init_plugin!(single::SingleChooserPlugin::new(&config.info.bands, props)),
        tracker::NAME => init_plugin!(tracker::TrackerChooserPlugin::new(config, props, gs_info)),
        _ => return Err(format!("{} is not a valid chooser plugin", name)),
    };

    Ok(chooser)
}
