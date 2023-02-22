use actix_web::http::header::ContentType;
use actix_web::web::Data;
use actix_web::{HttpRequest, HttpResponse};
use serde::Serialize;
use std::sync::RwLock;

use crate::state::{
    FrequencyStats, GroundStationMap, GroundStationStats, PositionReportsByFlightMap, SessionState,
};

pub async fn web_index(_req: HttpRequest) -> HttpResponse {
    HttpResponse::Ok()
        .content_type(ContentType::plaintext())
        .body("OK")
}

pub async fn api_gs_list(req: HttpRequest) -> HttpResponse {
    let gs_info = req.app_data::<Data<GroundStationMap>>().unwrap();

    HttpResponse::Ok()
        .content_type(ContentType::json())
        .body(serde_json::to_string(&gs_info).unwrap())
}

pub async fn api_gs_stats(req: HttpRequest) -> HttpResponse {
    let gs_stats = req.app_data::<Data<GroundStationStats>>().unwrap();

    HttpResponse::Ok()
        .content_type(ContentType::json())
        .body(serde_json::to_string(&gs_stats).unwrap())
}

pub async fn api_freq_stats(req: HttpRequest) -> HttpResponse {
    let freq_stats = req.app_data::<Data<FrequencyStats>>().unwrap();

    HttpResponse::Ok()
        .content_type(ContentType::json())
        .body(serde_json::to_string(&freq_stats).unwrap())
}

pub async fn api_session_list(req: HttpRequest) -> HttpResponse {
    let session_ptr = req.app_data::<Data<RwLock<SessionState>>>().unwrap();
    let session = session_ptr.read().unwrap();

    HttpResponse::Ok()
        .content_type(ContentType::json())
        .body(serde_json::to_string(&*session).unwrap())
}

#[derive(Debug, Serialize)]
struct FlightInfo {
    callsign: String,
    last_heard_on: Option<u32>,
    last_seen_secs: u64,
    path: Vec<Vec<f64>>,
}

pub async fn api_flights_list(req: HttpRequest) -> HttpResponse {
    let flight_posrpts = req.app_data::<Data<PositionReportsByFlightMap>>().unwrap();

    let summary: Vec<FlightInfo> = flight_posrpts
        .iter()
        .map(|x| {
            let val = x.value();

            FlightInfo {
                callsign: x.key().clone(),
                last_heard_on: val.positions.iter().last().map(|x| x.freq),
                last_seen_secs: val.last_heard.elapsed().as_secs(),
                path: val.positions.iter().map(|x| x.position.clone()).collect(),
            }
        })
        .collect();

    HttpResponse::Ok()
        .content_type(ContentType::json())
        .body(serde_json::to_string(&summary).unwrap())
}

#[derive(Debug, Serialize)]
struct PropagationPath {
    id: u8,
    name: String,
    bands: Vec<u32>,
    path: Vec<Vec<f64>>,
}

#[derive(Debug, Serialize)]
struct PositionReport {
    location: Vec<f64>,
    heard_on: u32,
    stations: Vec<PropagationPath>,
}

#[derive(Debug, Serialize)]
struct FlightDetail {
    callsign: String,
    last_seen_secs: u64,
    reports: Vec<PositionReport>,
}

pub async fn api_flights_detail(req: HttpRequest) -> HttpResponse {
    let flight_posrpts = req.app_data::<Data<PositionReportsByFlightMap>>().unwrap();
    let callsign = match req.match_info().get("callsign") {
        Some(val) => val,
        None => return HttpResponse::BadRequest().body("No flight provided"),
    };

    let flight = match flight_posrpts.get(callsign) {
        Some(val) => val,
        None => {
            return HttpResponse::NotFound().body(format!("Flight {} does not exist", callsign))
        }
    };

    let detail = FlightDetail {
        callsign: flight.key().to_string(),
        last_seen_secs: flight.last_heard.elapsed().as_secs(),
        reports: flight
            .positions
            .iter()
            .map(|x| PositionReport {
                location: x.position.clone(),
                heard_on: x.freq,
                stations: x
                    .propagation
                    .iter()
                    .map(|y| PropagationPath {
                        id: y.id,
                        name: y.name.clone(),
                        bands: y.bands.clone(),
                        path: vec![x.position.clone(), y.location.clone()],
                    })
                    .collect(),
            })
            .collect(),
    };

    HttpResponse::Ok()
        .content_type(ContentType::json())
        .body(serde_json::to_string(&detail).unwrap())
}
