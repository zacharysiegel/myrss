mod config;
mod db;
mod handlers;
mod models;
mod rss_fetcher;
mod templates;

use actix_files::Files;
use actix_session::{SessionMiddleware, storage::CookieSessionStore};
use actix_web::{web, App, HttpServer, middleware::Logger};
use actix_web::cookie::Key;
use anyhow::Result;
use sqlx::postgres::PgPoolOptions;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub db_pool: sqlx::PgPool,
    pub config: Arc<config::Config>,
}

#[actix_web::main]
async fn main() -> Result<()> {
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    let config = config::Config::from_env()?;
    
    let db_pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&config.database_url)
        .await?;

    // Run migrations
    sqlx::migrate!()
        .run(&db_pool)
        .await?;

    let state = AppState {
        db_pool,
        config: Arc::new(config.clone()),
    };

    log::info!("Starting server at http://{}:{}", config.host, config.port);

    HttpServer::new(move || {
        let session_key = Key::from(state.config.session_key.as_bytes());
        
        App::new()
            .app_data(web::Data::new(state.clone()))
            .wrap(Logger::default())
            .wrap(
                SessionMiddleware::builder(CookieSessionStore::default(), session_key)
                    .cookie_secure(false) // Set to true in production with HTTPS
                    .build()
            )
            .service(Files::new("/static", "./static").show_files_listing())
            .configure(handlers::configure)
    })
    .bind((config.host.as_str(), config.port))?
    .run()
    .await?;

    Ok(())
}