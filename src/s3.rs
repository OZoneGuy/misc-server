use actix_identity::Identity;
// use actix_identity::IdentityExt as _;
use actix_web::{
    get,
    web::{scope, Query, ServiceConfig},
    HttpResponse,
};

pub fn s3_config(cfg: &mut ServiceConfig) {
    cfg.service(scope("/s3").service(list_objects).service(get_object));
}

#[get("/list_objects")]
async fn list_objects(path: Option<Query<String>>, _id: Identity) -> HttpResponse {
    HttpResponse::Ok().body("list_objects")
}

#[get("/get_object")]
async fn get_object(file_path: Query<String>, _id: Identity) -> HttpResponse {
    HttpResponse::Ok().body("get_object")
}
