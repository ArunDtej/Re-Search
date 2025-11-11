use actix_web::{get, web, HttpResponse, Responder};

#[get("/search")]
pub async fn search(query: web::Query<SearchQuery>) -> impl Responder {
    let text = &query.text;
    HttpResponse::Ok().body(format!("You searched for: {}", text))
}

#[derive(serde::Deserialize)]
pub struct SearchQuery {
    pub text: String,
}
