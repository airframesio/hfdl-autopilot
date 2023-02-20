use actix_web::http::header::ContentType;
use actix_web::web::Data;
use actix_web::{HttpRequest, HttpResponse};

use crate::state::{
    FrequencyStats, GroundStationMap, GroundStationStats, PositionReportsByFlightMap,
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

pub async fn api_flights_list(req: HttpRequest) -> HttpResponse {
    let flight_posrpts = req.app_data::<Data<PositionReportsByFlightMap>>().unwrap();

    HttpResponse::Ok()
        .content_type(ContentType::json())
        .body(serde_json::to_string(&flight_posrpts).unwrap())
}
