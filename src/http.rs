use actix_web::{get, HttpRequest};

#[get("/")]
pub async fn index(req: HttpRequest) -> &'static str {
    "Hello, World\r\n"
}
