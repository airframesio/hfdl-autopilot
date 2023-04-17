use clap::Parser;
use log::*;
use serde_json::Value;
use std::env;
use std::io;
use std::process;
use std::process::Stdio;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::time;
use tokio::time::Instant;
use utils::arguments::Args;

mod autopilot;
mod utils;

#[tokio::main]
async fn main() -> io::Result<()> {
    let args = Args::parse();

    let verbose_level = match env::var("HFDLAP_DEBUG") {
        Ok(val) => {
            let norm = val.trim().to_lowercase();
            if norm.starts_with("t") || norm.starts_with("y") || norm.eq("1") {
                3
            } else {
                2
            }
        }
        Err(_) => 2,
    };

    stderrlog::new()
        .module(module_path!())
        .quiet(args.quiet)
        .verbosity(if args.verbose { verbose_level } else { 1 })
        .timestamp(if args.verbose {
            stderrlog::Timestamp::Second
        } else {
            stderrlog::Timestamp::Off
        })
        .init()
        .unwrap();

    if !args.bin.exists() {
        error!("Cannot find {}", args.bin.to_string_lossy());
        error!("Verify that dumphfdl exists in the path above and try again.");
        process::exit(1);
    }

    if !args.systable.exists() {
        error!("Cannot find {}", args.systable.to_string_lossy());
        error!("Verify that the systable configuration exists in the path above and try again.");
        process::exit(1);
    }

    let dumphfdl_args = if args.enable_feed_airframes {
        match utils::airframes::add_airframes_feeder_args(&args.hfdl_args) {
            Ok(p) => p,
            Err(e) => {
                error!(
                    "Error occurred when trying to enable airframes.io feeding: {}",
                    e.to_string()
                );
                error!("Make sure to add '--station-id' argument with a station name to the dumphfdl addition args, then try again.");
                process::exit(1)
            }
        }
    } else {
        args.hfdl_args.clone()
    };

    let (name, props) = args.chooser_config();

    let sampling_rates = match args.soapysdr_sample_rates() {
        Ok(r) => r,
        Err(e) => {
            error!(
                "Failed to determine valid SDR sampling rates: {}",
                e.to_string()
            );
            error!("Make sure '--soapysdr' argument is provided in additional arguments and try again.");
            process::exit(1)
        }
    };
    info!("supported sampling rates = {:?}", sampling_rates);

    let state = match autopilot::state::OpsState::init(
        &args.systable,
        args.max_bandwidth,
        sampling_rates,
        name,
        &props,
    ) {
        Ok(s) => s,
        Err(e) => {
            error!("Failed to initialize state: {}", e.to_string());
            error!("Check to make sure systable configuration path, max bandwidth value, and chooser plugin+arguments are valid and try again.");
            process::exit(1)
        }
    };
    info!("hfdl-autopilot settings");
    info!("  systable path = {}", args.systable.to_string_lossy());
    info!("  dumphfdl path = {}", args.bin.to_string_lossy());
    info!("  init max_bandwith = {}", args.max_bandwidth);

    if args.verbose && verbose_level == 3 {
        debug!("  hfdl freq bands");
        for (k, v) in &state.bands {
            debug!("    band {}: {:?}", k, v);
        }
    }

    info!("  init plugin name = {}, props = {:?}", name, props);

    let settings = autopilot::settings::Settings {
        use_airframes_live_gs: args.use_airframes_live_gs,
        stale_timeout_seconds: args.stale_timeout,
        session_break_seconds: args.session_break,
        session_timeout_seconds: args.session_timeout,
    };
    info!(
        "  enable_airframes_feeding = {}",
        args.enable_feed_airframes
    );
    info!(
        "  use_airframes_live_gs    = {}",
        args.use_airframes_live_gs
    );
    info!("  stale_timeout_seconds    = {}", args.stale_timeout);
    info!("  session_break_seconds    = {}", args.session_break);
    info!("  session_timeout_seconds  = {}", args.session_timeout);

    let mut app = autopilot::Autopilot::new(settings, state);

    if let Some(ref swarm_url) = args.swarm {
        app.enable_swarm(swarm_url);
        info!("enabling swarm with server at {}", swarm_url.to_string());
    } else {
        app.enable_api_server(
            &args.host,
            args.port,
            &args.token,
            args.disable_cors,
            args.disable_api_control,
        );
        info!(
            "enabling API server at http://{}:{}, use auth = {}, cors = {}, api_control = {}",
            args.host,
            args.port,
            !args.token.is_empty(),
            !args.disable_cors,
            !args.disable_api_control
        );
    }

    while app.should_run() {
        let (freqs, sampling_rate) = match app.choose_listening_freqs() {
            Ok((f, r)) => (f, r),
            Err(e) => {
                error!(
                    "Failed to choose next listening frequencies: {}",
                    e.to_string()
                );
                process::exit(1)
            }
        };

        let mut proc = match Command::new(&args.bin)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .arg("--system-table")
            .arg(&args.systable)
            .arg("--sample-rate")
            .arg(sampling_rate.to_string())
            .arg("--output")
            .arg("decoded:json:file:path=-")
            .args(&dumphfdl_args)
            .args(freqs.into_iter().map(|f| f.to_string()))
            .spawn()
        {
            Ok(p) => p,
            Err(e) => {
                error!("Failed to start dumphfdl: {}", e.to_string());
                continue;
            }
        };

        let child_stdout = match proc.stdout.take() {
            Some(o) => o,
            None => {
                error!("Unable to take STDOUT for child dumphfdl process");
                continue;
            }
        };
        let child_stderr = match proc.stderr.take() {
            Some(e) => e,
            None => {
                error!("Unable to take STDERR for child dumphfdl process");
                continue;
            }
        };

        let mut stdout_reader = BufReader::new(child_stdout);
        let mut stderr_reader = BufReader::new(child_stderr);

        let mut since_last_frame = Instant::now();
        let mut since_last_cleanup = Instant::now();

        loop {
            let mut msg = String::new();

            if let Ok(results) = time::timeout(
                Duration::from_millis(1000),
                stdout_reader.read_line(&mut msg),
            )
            .await
            {
                match results {
                    Ok(size) => {
                        if size == 0 {
                            // TODO: read stderr?

                            break;
                        }

                        let stale_timeout_secs: u64;
                        {
                            let settings = app.settings.read().unwrap();
                            stale_timeout_secs = settings.stale_timeout_seconds as u64;
                        }

                        let frame: Value = match serde_json::from_str(&msg) {
                            Ok(f) => f,
                            Err(e) => {
                                error!("HFDL frame is not valid JSON: {}", e.to_string());
                                continue;
                            }
                        };
                        since_last_frame = Instant::now();

                        if !args.disable_stdout {
                            println!("{}", msg.trim());
                        }

                        if app.on_frame(&frame) {
                            info!("NAME elects to change bands after last HFDL frame");
                            break;
                        }

                        let since_last_cleanup_secs = since_last_cleanup.elapsed().as_secs();
                        if since_last_cleanup_secs >= stale_timeout_secs {
                            // TODO: better info message for cleanup
                            info!("");
                            app.cleanup();
                            since_last_cleanup = Instant::now();
                        }
                    }
                    Err(e) => {
                        // TODO: we almost never run into this...

                        break;
                    }
                }
            } else {
                // TODO: get plugin name

                let session_timeout_seconds: u64;
                {
                    let settings = app.settings.read().unwrap();
                    session_timeout_seconds = settings.stale_timeout_seconds as u64;
                }

                let since_last_frame_secs = since_last_frame.elapsed().as_secs();
                if since_last_frame_secs > session_timeout_seconds && app.on_timeout() {
                    info!(
                        "been {} seconds since last HFDL frame heard, NAME elects to change bands",
                        since_last_frame_secs
                    );
                    break;
                }

                if app.should_change() {
                    // TODO: display better info message here
                    info!("");
                    break;
                }
            }
        }
    }

    Ok(())
}
