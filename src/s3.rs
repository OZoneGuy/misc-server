use actix_web::{
    get,
    web::{scope, Data, Query, ServiceConfig},
    HttpResponse,
};
use actix_web_lab::middleware::from_fn;
use aws_sdk_s3::Client;
use serde::Deserialize;

use crate::{auth::auth_middleware, common::Config};

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
struct S3Query {
    path: String,
}

pub fn s3_config(cfg: &mut ServiceConfig) {
    cfg.service(
        scope("/s3")
            .wrap(from_fn(auth_middleware))
            .service(list_objects)
            .service(get_object),
    );
}

#[get("/list_objects")]
async fn list_objects(
    path: Option<Query<S3Query>>,
    s3: Data<Client>,
    config: Data<Config>,
) -> HttpResponse {
    let prefix = &path.map(|p| p.path.clone()).unwrap_or("".to_string());

    let objects = s3
        .list_objects_v2()
        .bucket(&config.bucket_name)
        .delimiter("/")
        .prefix(prefix)
        .send()
        .await;
    HttpResponse::Ok().body(format!("objects: {:?}", objects.unwrap().common_prefixes()))
}

#[get("/get_object")]
async fn get_object(file_path: Option<Query<S3Query>>) -> HttpResponse {
    HttpResponse::Ok().body(format!("get_object: {:?}", file_path))
}
