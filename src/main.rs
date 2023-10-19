#![allow(clippy::needless_return)]
mod auth;
mod common;
mod errors;
mod index;
mod ip;
mod s3;

use actix_cors::Cors;

use actix_identity::IdentityMiddleware;
use actix_session::{storage::CookieSessionStore, SessionMiddleware};
use actix_web::{cookie::Key, http, middleware::Logger, web::Data, App, HttpServer};
use auth::auth_config;
use aws_credential_types::Credentials;
use aws_sdk_s3::config::{timeout::TimeoutConfig, Builder as S3Builder, Region};
use common::Config;
use env_logger::Env;
use index::index_config;
use ip::update_ip;
use log::{debug, info};
use s3::s3_config;

const SECRETS_JSON: &str = include_str!("../secrets.json");

#[derive(serde::Deserialize, Debug, Clone)]
struct Secrets {
    #[serde(rename = "NAME_CHEAP_API_KEY")]
    nc_api_key: String,
    #[serde(rename = "ENC_KEY")]
    key: String,
    #[serde(rename = "AWS_ACCESS_KEY")]
    aws_access_key: String,
    #[serde(rename = "AWS_SECRET_ACCESS_KEY")]
    aws_secret_access_key: String,
}

impl Secrets {
    fn aws_creds(&self) -> Credentials {
        Credentials::from_keys(
            self.aws_access_key.clone(),
            self.aws_secret_access_key.clone(),
            None,
        )
    }
}

async fn create_s3_client(provider: &Secrets) -> aws_sdk_s3::Client {
    let config = aws_config::from_env()
        .region(Region::new("us-east-1"))
        .credentials_provider(provider.aws_creds())
        .load()
        .await;

    let timeout_config = TimeoutConfig::builder()
        // .connect_timeout(Duration::from_secs(7))
        .build();

    let s3_config = S3Builder::from(&config)
        .timeout_config(timeout_config)
        .build();

    aws_sdk_s3::Client::from_conf(s3_config)
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init_from_env(Env::default().default_filter_or("debug"));

    info!("Entering main");

    let secrets: Secrets =
        serde_json::from_str(SECRETS_JSON).expect("Failed to parse the secrets json");
    info!("Got secrets");

    let mut server_ip = std::fs::read_to_string("/tmp/current_ip.txt").unwrap_or("".to_string());
    // TODO: cache the results. This causes the largest delay in the startup
    if server_ip == "" {
        server_ip = reqwest::get("https://api.ipify.org")
            .await
            .expect("Failed to create request to get the server IP")
            .text()
            .await
            .expect("Failed to get the IP from request bodY");
    };
    info!("Got server IP: {}", &server_ip);

    let config = Config {
        nc_api_key: secrets.nc_api_key.clone(),
        server_ip,
        bucket_name: "unraid-remote-sync".into(),
    };
    debug!("Config: {}, {}", &config.server_ip, &config.bucket_name);

    let s3_client = create_s3_client(&secrets).await;
    info!("Created S3 client");

    info!("Starting server");
    HttpServer::new(move || {
        let key = Key::from(secrets.key.as_bytes());

        let cors = if cfg!(debug_assertions) {
            debug!("Permissive CORS");
            Cors::permissive()
        } else {
            debug!("Strict CORS");
            Cors::default()
                .allowed_origin_fn(|origin, _req_head| {
                    origin.as_bytes().ends_with(b"omaralkersh.com")
                })
                .allowed_methods(vec!["GET", "POST"])
                .allowed_headers(vec![
                    http::header::AUTHORIZATION,
                    http::header::ACCEPT,
                    http::header::CONTENT_TYPE,
                ])
                .supports_credentials()
        };

        App::new()
            .wrap(cors)
            .wrap(IdentityMiddleware::default())
            .wrap(
                SessionMiddleware::builder(CookieSessionStore::default(), key)
                    .cookie_secure(true)
                    .build(),
            )
            .wrap(Logger::default())
            .app_data(Data::new(config.clone()))
            .app_data(Data::new(s3_client.clone()))
            .configure(s3_config)
            .service(update_ip)
            .configure(index_config)
            .configure(auth_config)
    })
    .bind(("localhost", 8123))?
    .run()
    .await
}
