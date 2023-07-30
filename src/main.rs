#![allow(clippy::needless_return)]
use actix_web::{get, post, web, App, HttpResponse, HttpServer, Responder};

const SECRETS_JSON: &str = include_str!("../secrets.json");
const OLD_IP_PATH: &str = "/tmp/old_ip.txt";

#[derive(serde::Deserialize)]
struct Secrets {
    #[serde(rename = "NAME_CHEAP_API_KEY")]
    nc_api_key: String,
}

#[derive(Clone)]
struct Config {
    nc_api_key: String,
    server_ip: String,
}

#[derive(serde::Deserialize)]
struct UpdateIpRequest {
    ip: String,
}

#[post("/update-ip")]
async fn update_ip(req: web::Json<UpdateIpRequest>, config: web::Data<Config>) -> impl Responder {
    // Check if the IP has changed
    let old_ip = std::fs::read_to_string(OLD_IP_PATH).unwrap_or("".to_string());
    if old_ip == req.ip {
        return HttpResponse::Ok().body("IP has not changed");
    }

    // load params to send to the request
    let params = [
        ("apiUser", "OZonePerson"),
        ("apiKey", &config.nc_api_key),
        ("ClientIp", &config.server_ip),
        ("username", "OZonePerson"),
        ("hostName", "home.ozoneperson.com"),
        ("Command", "namecheap.domains.dns.setHosts"),
        ("SLD", "omaralkersh"),
        ("TLD", "com"),
        // The base domain
        ("HostName1", "@"),
        ("RecordType1", "A"),
        ("Address1", &config.server_ip),
        ("TTL1", "1800"),
        // The www subdomain
        ("HostName1", "www"),
        ("RecordType1", "A"),
        ("Address1", &config.server_ip),
        ("TTL1", "1800"),
        // The VPN subdomain
        ("HostName2", "vpn"),
        ("RecordType2", "A"),
        ("Address2", &req.ip),
        ("TTL2", "1800"),
    ];

    // Create request to namecheap to update the IP
    let client = reqwest::Client::new();
    let res = client
        .post("https://api.namecheap.com/xml.response")
        .form(&params)
        .send()
        .await;

    return match res {
        Ok(_) => {
            // Update the old IP file
            std::fs::write(OLD_IP_PATH, &req.ip).unwrap();

            return HttpResponse::Ok().body(format!("Updated IP to {}", &req.ip));
        }
        Err(_) => HttpResponse::InternalServerError().body("Failed to update IP"),
    };
}

#[get("/health")]
async fn health() -> impl Responder {
    HttpResponse::Ok().body("OK")
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

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(config.clone()))
            .service(update_ip)
            .service(health)
    })
    .bind(("localhost", 8123))?
    .run()
    .await
}
