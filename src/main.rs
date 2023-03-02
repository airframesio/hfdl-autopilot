use crate::state::SharedState;
use actix_web::{rt, web, App, HttpServer};
use clap::Parser;
use log::*;
use serde_json::Value;
use std::io;
use std::io::{Seek, SeekFrom, Write};
use std::process::Stdio;
use std::time::{Duration, Instant};
use tempfile::NamedTempFile;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::time;

mod args;
mod chooser;
mod config;
mod hfdl;
mod http;
mod state;
mod utils;

#[tokio::main]
async fn main() -> io::Result<()> {
    let args = args::Args::parse();

    stderrlog::new()
        .module(module_path!())
        .quiet(args.quiet)
        .verbosity(if args.verbose { 3 } else { 1 })
        .timestamp(if args.verbose {
            stderrlog::Timestamp::Second
        } else {
            stderrlog::Timestamp::Off
        })
        .init()
        .unwrap();

    let config = match config::Config::from_args(&args) {
        Ok(cfg) => cfg,
        Err(e) => {
            error!("Failed to parse configuration: {}", e);
            return Ok(());
        }
    };
    info!("Configuration demarshalled from command line arguments.");
    info!("  {}", config);

    let (name, props) = args.chooser_params();
    info!("Chooser plugin name={} props={:?}", name, props);

    let mut shared_state = SharedState::new(&config);

    let mut plugin = match chooser::get(name, &config, &props, shared_state.gs_info.clone()) {
        Ok(plugin) => plugin,
        Err(e) => {
            error!("PLUGIN INIT[{}]: {}", name, e);
            return Ok(());
        }
    };

    if config.swarm {
        info!("Swarm mode is ON: target={}:{}", config.host, config.port);
        error!("UNSUPPORTED for now...");

        // TODO: what does swarm mode look like
    } else {
        info!(
            "Swarm mode is OFF: starting web server on {}:{}",
            config.host, config.port
        );

        let session = shared_state.session.clone();
        let gs_info = shared_state.gs_info.clone();
        let gs_stats = shared_state.gs_stats.clone();
        let flight_posrpt = shared_state.flight_posrpt.clone();
        let freq_stats = shared_state.freq_stats.clone();

        let server_host = config.host.clone();
        let server_port = config.port;

        tokio::spawn(async move {
            let server = HttpServer::new(move || {
                App::new()
                    .app_data(session.clone())
                    .app_data(gs_info.clone())
                    .app_data(gs_stats.clone())
                    .app_data(flight_posrpt.clone())
                    .app_data(freq_stats.clone())
                    .route("/", web::get().to(http::web_index))
                    .route("/api/session", web::get().to(http::api_session_list))
                    .route("/api/ground-stations", web::get().to(http::api_gs_list))
                    .route(
                        "/api/ground-station/stats",
                        web::get().to(http::api_gs_stats),
                    )
                    .route("/api/freq-stats", web::get().to(http::api_freq_stats))
                    .route("/api/flights", web::get().to(http::api_flights_list))
                    .route(
                        "/api/flight/{callsign}",
                        web::get().to(http::api_flights_detail),
                    )
            })
            .bind((server_host, server_port))
            .unwrap()
            .run();
            server.await
        });
    }

    let mut systable = NamedTempFile::new()?;
    write!(systable, "{}", config.info.raw)?;
    systable.seek(SeekFrom::Start(0))?;

    let systable_temp_path = systable.into_temp_path();
    let timeout = Duration::from_secs(config.timeout as u64);

    info!(
        "System Table information written to {:?}",
        systable_temp_path
    );
    info!("Starting listening session...");
    info!("");

    let mut bad_child_reads = 0;

    while bad_child_reads < config.max_bad_child_reads {
        let band = match plugin.choose() {
            Ok(val) => val.to_owned(),
            Err(e) => {
                error!("Failed to choose a frequency band to listen to: {}", e);
                return Ok(());
            }
        };

        shared_state.update_current_band(&band);

        let bandwidth = match band.iter().max().unwrap_or(&0) - band.iter().min().unwrap_or(&0) {
            d if d >= 452 && d < 764 => "768000",
            d if d >= 380 && d < 452 => "456000",
            d if d > 252 && d < 380 => "384000",
            d if d <= 252 => "256000",
            _ => {
                error!("Bandwidth calculation failed: {:?}", band);
                return Ok(());
            }
        };

        info!("NEW SESSION: sample_rate={} band={:?}", bandwidth, band);

        let mut proc = match Command::new(config.bin.clone())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .arg("--system-table")
            .arg(systable_temp_path.to_path_buf())
            .arg("--sample-rate")
            .arg(bandwidth)
            .arg("--output")
            .arg("decoded:json:file:path=-")
            .args(config.additional_args.clone())
            .args(band.into_iter().map(|f| f.to_string()))
            .spawn()
        {
            Ok(proc) => proc,
            Err(e) => {
                error!("Failed to start dumphfdl: {}", e);
                continue;
            }
        };

        let child_stdout = match proc.stdout.take() {
            Some(stdout) => stdout,
            None => {
                error!("Unable to get STDOUT for child dumphfdl process!");
                continue;
            }
        };
        let mut reader = BufReader::new(child_stdout);
        let mut last_cleanup = Instant::now();

        loop {
            let mut msg = String::new();

            if let Ok(results) = rt::time::timeout(timeout, reader.read_line(&mut msg)).await {
                match results {
                    Ok(size) => {
                        if size == 0 {
                            error!("Read error: encountered 0 sized read from dumphfdl! (attempt {} of {})", bad_child_reads + 1, config.max_bad_child_reads);
                            bad_child_reads += 1;
                            break;
                        }

                        let frame: Value = match serde_json::from_str(&msg) {
                            Ok(val) => val,
                            Err(e) => {
                                error!("Bad JSON decode: {}", e);
                                continue;
                            }
                        };

                        println!("{}", msg.trim());

                        shared_state.update(&frame);

                        if plugin.on_recv_frame(&frame) {
                            info!("{} elects to change bands after last HFDL frame.", name);
                            break;
                        }

                        if last_cleanup.elapsed().as_secs() >= config.ac_timeout {
                            shared_state.clean_up();
                            last_cleanup = Instant::now();
                        }
                    }
                    Err(e) => {
                        error!(
                            "Read error: {} (attempt {} of {})",
                            e,
                            bad_child_reads + 1,
                            config.max_bad_child_reads
                        );

                        bad_child_reads += 1;
                        break;
                    }
                }
            } else if plugin.on_timeout() {
                info!(
                    "Been {}s since last message on band. {} elects to change bands.",
                    config.timeout, name
                );
                break;
            }
        }

        proc.kill().await?;

        if bad_child_reads < config.max_bad_child_reads && config.end_session_wait > 0 {
            info!(
                "Waiting {} seconds before starting new session",
                config.end_session_wait
            );
            time::sleep(Duration::from_secs(config.end_session_wait)).await;
        }

        info!("Ending session...");

        shared_state.clean_up();

        info!("");
    }

    if bad_child_reads > 0 {
        error!("Encountered read errors from dumphfdl: process may be prematurely exiting");
        error!(
            "Verify that dumphfdl is being fed with correct arguments and can be run indepedently"
        );
    }

    Ok(())
}
