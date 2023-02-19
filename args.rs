use clap::Parser;
use std::{collections::HashMap, path::PathBuf};

#[derive(Parser, Debug)]
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
    #[arg(long, value_name = "FILEPATH", default_value = "/etc/systable.json")]
    pub sys_table: PathBuf,

    /// Timeout in seconds to wait before switching HF bands
    #[arg(short, long, value_name = "SECONDS", default_value_t = 150)]
    pub timeout: u32,

    /// Methodology for changing HFDL bands
    #[arg(
        long,
        value_name = "PLUGIN_NAME[:KEY=VALUE,...]",
        default_value = "single:band=13"
    )]
    pub chooser: String,

    pub additional_args: Vec<String>,
}

impl Args {
    pub fn chooser_params(&self) -> (&str, HashMap<&str, &str>) {
        let mut props: HashMap<&str, &str> = HashMap::new();

        let delim = match self.chooser.find(":") {
            Some(val) => val,
            None => return (&self.chooser, props),
        };

        let name = &self.chooser[..delim];

        for kv in self.chooser[(delim + 1)..].split(",") {
            let delim = match kv.find("=") {
                Some(val) => val,
                None => {
                    props.insert(kv, "");
                    continue;
                }
            };

            props.insert(&kv[..delim], &kv[(delim + 1)..]);
        }

        (name, props)
    }
}
