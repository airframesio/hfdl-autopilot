use crossbeam::channel::{after, bounded, select};
use serde_json::Value;
use std::io::{BufRead, BufReader, Seek, SeekFrom, Write};
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;
use tempfile::NamedTempFile;

use clap::Parser;
use log::*;

mod args;
mod chooser;
mod config;

fn main() {
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
            error!("Failed to parser configuration: {}", e);
            return;
        }
    };
    info!("Configuration demarshalled from command line arguments.");
    info!("  {}", config);

    let (name, props) = args.chooser_params();
    info!("Chooser plugin name={} props={:?}", name, props);

    let mut plugin = match chooser::get(name) {
        Some(plugin) => plugin,
        None => {
            error!("Invalid plugin name: {}", name);
            return;
        }
    };

    let mut systable = match NamedTempFile::new() {
        Ok(fd) => fd,
        Err(e) => {
            error!("Unable to create temporary file for systable config: {}", e);
            return;
        }
    };
    write!(systable, "{}", config.info.raw).unwrap();
    systable.seek(SeekFrom::Start(0)).unwrap();

    let systable_temp_path = systable.into_temp_path();

    let mut addition_args: Vec<String> = vec![];
    if let Some(output) = config.output {
        addition_args.extend_from_slice(&["--output".to_string(), output])
    }

    info!(
        "System Table information written to {:?}",
        systable_temp_path
    );
    info!("Starting listening session...");
    info!("");

    loop {
        let band = match plugin.choose(&config.info.bands, &props) {
            Ok(val) => val.clone(),
            Err(e) => {
                error!("Failed to choose a frequency band to listen to: {}", e);
                return;
            }
        };
        info!("New session started: band={:?}", band);

        let bandwidth = match band.iter().max().unwrap_or(&0) - band.iter().min().unwrap_or(&0) {
            d if d > 256 && d <= 384 => "384000",
            d if d <= 256 => "256000",
            _ => {
                error!("Bandwidth calculation failed: {:?}", band);
                return;
            }
        };

        let mut proc = match Command::new(config.bin.clone())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .arg("--soapysdr")
            .arg(config.driver.clone())
            .arg("--system-table")
            .arg(systable_temp_path.to_path_buf())
            .arg("--sample-rate")
            .arg(bandwidth)
            .arg("--output")
            .arg("decoded:json:file:path=-")
            .args(&addition_args)
            .args(band.into_iter().map(|f| f.to_string()))
            .spawn()
        {
            Ok(proc) => proc,
            Err(e) => {
                error!("Failed to start dumphfdl: {}", e);
                continue;
            }
        };

        let (frame_send, frame_recv) = bounded(2048);
        let child_stdout = match proc.stdout.take() {
            Some(stdout) => stdout,
            None => {
                error!("Unable to get STDOUT for child dumphfdl process!");
                continue;
            }
        };
        let reader_thread = thread::spawn(move || {
            let mut reader = BufReader::new(child_stdout);

            loop {
                let mut line = String::new();
                let size = match reader.read_line(&mut line) {
                    Ok(size) => size,
                    Err(e) => {
                        error!("Reader thread encountered read error: {}", e);
                        break;
                    }
                };
                if size == 0 {
                    error!("Reader thread encountered empty read: exiting...");
                    break;
                }

                if frame_send.send(line).is_err() {
                    error!("Reader thread failed to send to main thread: exiting...");
                    break;
                }
            }
        });

        let timeout = Duration::from_secs(config.timeout as u64);

        loop {
            select! {
                recv(frame_recv) -> msg => {
                    if msg.is_ok() {
                        let msg = msg.unwrap();

                        let frame: Value = match serde_json::from_str(&msg) {
                            Ok(val) => val,
                            Err(e) => {
                                error!("Bad JSON decode: {}", e);
                                continue;
                            },
                        };

                        info!("Received {} byte frame...", msg.len());
                        println!("{}", msg.trim());

                        if plugin.on_update(&frame) {
                            info!("Chooser update elected to change bands...");
                            break;
                        }
                    }
                },
                recv(after(timeout)) -> _ => {
                    if plugin.on_timeout() {
                        info!("Timeout! Chooser elected to change bands...");
                        break;
                    }
                },
            }
        }

        proc.kill().unwrap();
        reader_thread.join().unwrap();

        info!("Ending session...");
        info!("");
    }
}
