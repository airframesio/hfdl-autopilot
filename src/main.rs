use actix_web::{rt, App, HttpServer};
use clap::Parser;
use log::*;
use serde_json::Value;
use std::io;
use std::io::{Seek, SeekFrom, Write};
use std::process::Stdio;
use std::time::Duration;
use tempfile::NamedTempFile;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

mod args;
mod chooser;
mod config;
mod http;

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

    let mut plugin = match chooser::get(name, &config.info.bands, &props) {
        Some(plugin) => plugin,
        None => {
            error!("Invalid plugin name: {}", name);
            return Ok(());
        }
    };

    if config.swarm {
        info!("Swarm mode is ON: target={}:{}", config.host, config.port)
    } else {
        info!(
            "Swarm mode is OFF: starting web server on {}:{}",
            config.host, config.port
        );
        tokio::spawn(async move {
            let server = HttpServer::new(|| App::new().service(http::index))
                .bind((config.host, config.port))
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

        info!("New session started: band={:?}", band);

        let bandwidth = match band.iter().max().unwrap_or(&0) - band.iter().min().unwrap_or(&0) {
            d if d > 256 && d <= 384 => "384000",
            d if d <= 256 => "256000",
            _ => {
                error!("Bandwidth calculation failed: {:?}", band);
                return Ok(());
            }
        };

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

                        info!("frame: {:?}", frame);
                        println!("{}", msg.trim());

                        if plugin.on_recv_frame(&frame) {
                            info!("{} elects to change bands after last HFDL frame.", name);
                            break;
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

        info!("Ending session...");
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
