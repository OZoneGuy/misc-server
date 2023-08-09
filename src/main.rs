#![allow(clippy::needless_return)]
mod auth;
mod common;
mod ip;
mod s3;

use actix_identity::IdentityMiddleware;
use actix_session::{storage::CookieSessionStore, SessionMiddleware};
use actix_web::{
    cookie::Key,
    get,
    middleware::Logger,
    web::{Data, ServiceConfig},
    App, HttpResponse, HttpServer, Responder,
};
use auth::auth_config;
use common::Config;
use env_logger::Env;
use ip::update_ip;
use s3::s3_config;

const SECRETS_JSON: &str = include_str!("../secrets.json");

#[derive(serde::Deserialize)]
struct Secrets {
    #[serde(rename = "NAME_CHEAP_API_KEY")]
    nc_api_key: String,
}

#[derive(serde::Serialize)]
struct Version {
    version: String,
    commit: String,
}

#[get("/health")]
async fn health() -> impl Responder {
    HttpResponse::Ok().body("OK")
}

#[get("/version")]
async fn version() -> impl Responder {
    let version = Version {
        version: env!("CARGO_PKG_VERSION").to_string(),
        commit: option_env!("GH_SHA_REF")
            .unwrap_or("not_commit")
            .to_string(),
    };
    HttpResponse::Ok().json(version)
}

#[get("/")]
async fn index() -> impl Responder {
    HttpResponse::Ok().body("This is an API server for personal use.")
}

fn index_config(cfg: &mut ServiceConfig) {
    cfg.service(index).service(version).service(health);
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let secrets: Secrets = serde_json::from_str(SECRETS_JSON).unwrap();
    let server_ip: String = reqwest::get("https://api.ipify.org")
        .await
        .unwrap()
        .text()
        .await
        .unwrap();

    let config = Config {
        nc_api_key: secrets.nc_api_key,
        server_ip,
    };

    env_logger::init_from_env(Env::default().default_filter_or("info"));

    HttpServer::new(move || {
        let key = Key::generate();
        App::new()
            .app_data(Data::new(config.clone()))
            .configure(s3_config)
            .service(update_ip)
            .configure(index_config)
            .configure(auth_config)
            .wrap(IdentityMiddleware::default())
            .wrap(SessionMiddleware::new(CookieSessionStore::default(), key))
            .wrap(Logger::default())
    })
    .bind(("localhost", 8123))?
    .run()
    .await
}
