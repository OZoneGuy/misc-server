// use actix_identity::IdentityExt as _;
use actix_web::{
    get,
    web::{scope, Query, ServiceConfig},
    HttpResponse,
};
use actix_web_lab::middleware::from_fn;

use crate::auth::auth_middleware;

pub fn s3_config(cfg: &mut ServiceConfig) {
    cfg.service(
        scope("/s3")
            .wrap(from_fn(auth_middleware))
            .service(list_objects)
            .service(get_object),
    );
}

#[get("/list_objects")]
async fn list_objects(path: Option<Query<String>>) -> HttpResponse {
    HttpResponse::Ok().body(format!("list_objects: {:?}", path))
}

#[get("/get_object")]
async fn get_object(file_path: Query<String>) -> HttpResponse {
    HttpResponse::Ok().body(format!("get_object: {:?}", file_path))
}
