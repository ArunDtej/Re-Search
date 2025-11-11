use actix_cors::Cors;
use actix_web::{middleware, web, App, HttpServer};
use tracing_actix_web::TracingLogger;
use tracing_subscriber;

#[path = "../api/search.rs"]
mod search;
#[path = "../api/health.rs"]
mod health;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    tracing_subscriber::fmt::init();

    println!("🚀 Starting API server at http://0.0.0.0:8080");

    HttpServer::new(|| {
        App::new()
            .wrap(TracingLogger::default())
            .wrap(middleware::ErrorHandlers::default())
            .wrap(
                Cors::default()
                    .allow_any_origin()
                    .allowed_methods(vec!["GET", "POST"])
                    .max_age(3600),
            )
            .wrap(middleware::Compress::default())
            // ✅ Register endpoints directly
            .service(search::search)
            .service(health::health)
    })
    .workers(num_cpus::get())
    .bind(("0.0.0.0", 8080))?
    .run()
    .await
}
