#[derive(Debug)]
pub struct Settings {
    pub use_airframes_live_gs: bool,

    pub stale_timeout_seconds: u32,
    pub session_break_seconds: u32,
    pub session_timeout_seconds: u32,
}
