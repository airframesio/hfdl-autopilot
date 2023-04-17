use clap::Parser;
use std::collections::HashMap;
use std::io;
use std::path::PathBuf;
use url::Url;

#[derive(Debug, Parser)]
pub struct Args {
    /// Verbose mode
    #[arg(short, long, default_value_t = false)]
    pub verbose: bool,

    /// Silence all output
    #[arg(short, long, default_value_t = false)]
    pub quiet: bool,

    /// Path to dumphfdl binary
    #[arg(long, value_name = "FILEPATH", default_value = "/usr/bin/dumphfdl")]
    pub bin: PathBuf,

    /// Path to dumphfdl system table configuration file
    #[arg(long, value_name = "FILEPATH", default_value = "/etc/systable.conf")]
    pub systable: PathBuf,

    /// Maximum bandwidth to use for splitting HFDL spectrum into bands of coverage
    #[arg(long = "max-bandwidth", value_name = "HZ", default_value_t = 512000)]
    pub max_bandwidth: u32,

    /// When provided, hfdl-autopilot connects to a swarm leader to collate data and band control
    #[arg(long, value_name = "URL", default_value = None)]
    pub swarm: Option<Url>,

    /// Host for API server to listen on
    #[arg(long = "api-host", value_name = "HOST", default_value = "0.0.0.0")]
    pub host: String,

    /// Port for API server to listen on
    #[arg(long = "api-port", value_name = "PORT", default_value_t = 7270)]
    pub port: u16,

    /// Set an authentication token for API server access
    #[arg(long = "api-token", value_name = "AUTH-TOKEN", default_value = "")]
    pub token: String,

    /// Use airframes.io's live ground station frequency map  
    #[arg(long = "use-airframes-live-gs", default_value_t = false)]
    pub use_airframes_live_gs: bool,

    /// Enable feeding of HFDL frames to airframes.io
    #[arg(long = "enable-feed-airframes", default_value_t = false)]
    pub enable_feed_airframes: bool,

    /// Disable cross site request sharing (CORS)
    #[arg(long = "disable-cors", default_value_t = false)]
    pub disable_cors: bool,

    /// Disable API server from being able to control hfdl-autopilot
    #[arg(long = "disable-api-control", default_value_t = false)]
    pub disable_api_control: bool,

    /// Disable printing JSON HFDL frames to STDOUT
    #[arg(long = "disable-stdout", default_value_t = false)]
    pub disable_stdout: bool,

    /// Initial chooser plugin to change HFDL bands
    #[arg(long, value_name = "PLUGIN-NAME", default_value = "single")]
    chooser: String,

    /// Initial chooser plugin arguments
    #[arg(long, value_name = "PLUGIN-ARGS", default_value = "band=13")]
    chooser_args: String,

    /// Elapsed time since last update before an aircraft and ground station frequency data is considered stale
    #[arg(long = "stale-timeout", value_name = "SECONDS", default_value_t = 900)]
    pub stale_timeout: u32,

    /// Time to wait after terminating dumphfdl before starting a new session
    #[arg(long = "session-break", value_name = "SECONDS", default_value_t = 0)]
    pub session_break: u32,

    /// Elapsed time since last HFDL frame before a session is considered stale and requires switching
    #[arg(
        short = 't',
        long = "session-timeout",
        value_name = "SECONDS",
        default_value_t = 0
    )]
    pub session_timeout: u32,

    /// Additional arguments for dumphfdl
    pub hfdl_args: Vec<String>,
}

impl Args {
    pub fn chooser_config(&self) -> (&str, HashMap<&str, &str>) {
        let mut props: HashMap<&str, &str> = HashMap::new();

        for kv in self.chooser_args.split(",") {
            let delim = match kv.find("=") {
                Some(val) => val,
                None => {
                    props.insert(kv, "");
                    continue;
                }
            };
            props.insert(&kv[..delim], &kv[(delim + 1)..]);
        }

        (&self.chooser, props)
    }

    pub fn soapysdr_sample_rates(&self) -> io::Result<Vec<u32>> {
        let soapy_opts: Vec<&str> = match self
            .hfdl_args
            .iter()
            .position(|x| x.eq_ignore_ascii_case("--soapysdr"))
        {
            Some(idx) => {
                if idx + 1 < self.hfdl_args.len() {
                    self.hfdl_args[idx + 1].split(",").collect()
                } else {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        format!(
                            "Malformed SoapySDR string, missing values after '--soapysdr': {:?}",
                            self.hfdl_args
                        ),
                    ));
                }
            }
            None => {
                return Err(io::Error::new(
                    io::ErrorKind::NotFound,
                    format!(
                        "Missing '--soapysdr' argument in dumphfdl arguments: {:?}",
                        self.hfdl_args
                    ),
                ))
            }
        };

        let driver = match soapy_opts.iter().find(|x| x.starts_with("driver=")) {
            Some(val) => *val,
            None => {
                return Err(io::Error::new(
                    io::ErrorKind::NotFound,
                    format!(
                        "SoapySDR driver options missing 'driver=': {:?}",
                        soapy_opts
                    ),
                ))
            }
        };

        // TODO: Make rust-soapysdr call to get sample rates here
        // TODO: check to make sure Vec is populated
        // TODO: Make sure to sort in ascending order

        Ok(vec![250000, 500000, 5000000, 6000000])
    }
}
