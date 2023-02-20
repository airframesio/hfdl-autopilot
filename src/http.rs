use actix_web::http::header::ContentType;
use actix_web::web::Data;
use actix_web::{get, HttpRequest, HttpResponse};

use crate::state::GroundStationMap;

pub async fn web_index(req: HttpRequest) -> HttpResponse {
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
