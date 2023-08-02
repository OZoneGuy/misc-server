#![allow(clippy::needless_return)]
use std::borrow::Cow;

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

#[derive(serde::Serialize)]
struct Version {
    version: String,
    commit: String,
}

fn record(number: usize, sub_domain: Cow<str>, ip: Cow<str>) -> Vec<(String, String)> {
    return vec![
        (format!("HostName{}", number), sub_domain.to_string()),
        (format!("RecordType{}", number), "A".to_string()),
        (format!("Address{}", number), ip.to_string()),
        (format!("TTL{}", number), "1800".to_string()),
    ];
}

fn create_request(
    server_ip: Cow<str>,
    new_ip: Cow<str>,
    nc_api_key: Cow<str>,
) -> Vec<(String, String)> {
    let mut params: Vec<(String, String)> = vec![
        ("apiUser", "OZonePerson"),
        ("apiKey", &nc_api_key),
        ("ClientIp", &server_ip),
        ("username", "OZonePerson"),
        ("hostName", "home.ozoneperson.com"),
        ("Command", "namecheap.domains.dns.setHosts"),
        ("SLD", "omaralkersh"),
        ("TLD", "com"),
    ]
    .into_iter()
    .map(|(k, v)| (k.to_string(), v.to_string()))
    .collect();

    vec![("@", &server_ip), ("www", &server_ip), ("vpn", &new_ip)]
        .into_iter()
        .enumerate()
        .for_each(|(i, (sub_domain, ip))| {
            params.append(&mut record(i + 1, sub_domain.into(), ip.clone()));
        });

    return params;
}

#[post("/update-ip")]
async fn update_ip(req: web::Json<UpdateIpRequest>, config: web::Data<Config>) -> impl Responder {
    // Check if the IP has changed
    let old_ip = std::fs::read_to_string(OLD_IP_PATH).unwrap_or("".to_string());
    if old_ip == req.ip {
        return HttpResponse::Ok().body("IP has not changed");
    }

    // load params to send to the request
    let params = create_request(
        Cow::from(&config.server_ip),
        Cow::from(&req.ip),
        Cow::from(&config.nc_api_key),
    );

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
            .service(index)
            .service(version)
            .service(health)
            .service(update_ip)
    })
    .bind(("localhost", 8123))?
    .run()
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_record() {
        let address = "1235";

        let expected: Vec<(String, String)> = vec![
            ("HostName1", "@"),
            ("RecordType1", "A"),
            ("Address1", address),
            ("TTL1", "1800"),
        ]
        .into_iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();

        let actual = record(1, "@".into(), address.into());

        assert_eq!(expected, actual);
    }

    #[test]
    fn test_create_request() {
        let server_ip = "1234";
        let new_ip = "1235";
        let nc_api_key = "1236";

        let expected: Vec<(String, String)> = vec![
            ("apiUser", "OZonePerson"),
            ("apiKey", nc_api_key),
            ("ClientIp", server_ip),
            ("username", "OZonePerson"),
            ("hostName", "home.ozoneperson.com"),
            ("Command", "namecheap.domains.dns.setHosts"),
            ("SLD", "omaralkersh"),
            ("TLD", "com"),
            ("HostName1", "@"),
            ("RecordType1", "A"),
            ("Address1", server_ip),
            ("TTL1", "1800"),
            ("HostName2", "www"),
            ("RecordType2", "A"),
            ("Address2", server_ip),
            ("TTL2", "1800"),
            ("HostName3", "vpn"),
            ("RecordType3", "A"),
            ("Address3", new_ip),
            ("TTL3", "1800"),
        ]
        .into_iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();

        let actual = create_request(server_ip.into(), new_ip.into(), nc_api_key.into());

        assert_eq!(expected, actual);
    }
}
