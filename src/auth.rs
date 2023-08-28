use actix_identity::{Identity, IdentityExt};
use actix_web::{
    body::MessageBody,
    dev::{ServiceRequest, ServiceResponse},
    get,
    http::StatusCode,
    post,
    web::{Json, ServiceConfig},
    Error, HttpMessage, HttpRequest, HttpResponse, HttpResponseBuilder, Responder,
};
use actix_web_lab::middleware::Next;
use ldap3::LdapConn;
use serde::{Deserialize, Serialize};

use crate::errors::{Result, ServerError};

pub fn auth_config(cfg: &mut ServiceConfig) {
    cfg.service(login).service(logout).service(user);
}

#[derive(Deserialize, Debug)]
pub struct LoginRequest {
    username: String,
    password: String,
}

#[derive(Serialize, Debug)]
pub struct User(String);

#[post("/login")]
pub async fn login(req: HttpRequest, login_details: Json<LoginRequest>) -> Result<HttpResponse> {
    // login to ldap server and get user_id
    let mut ldap = LdapConn::new("ldap://localhost:389").map_err(|e| ServerError::LoginError {
        code: StatusCode::INTERNAL_SERVER_ERROR,
        message: format!("Failed to connect to ldap server: {}", e.to_string()),
    })?;
    let bound = ldap
        .simple_bind(&login_details.username, &login_details.password)
        .map_err(|e| ServerError::LoginError {
            code: StatusCode::UNAUTHORIZED,
            message: e.to_string(),
        })?;

    Identity::login(&req.extensions(), bound.matched).map_err(|e| ServerError::LoginError {
        code: StatusCode::INTERNAL_SERVER_ERROR,
        message: format!("Failed to login: {}", e.to_string()),
    })?;

    Ok(HttpResponse::Ok().body("Logged in"))
}

#[post("/logout")]
pub async fn logout(id: Identity) -> impl Responder {
    id.logout();

    HttpResponse::Ok()
}

#[get("/user")]
pub async fn user(id: Option<Identity>) -> Result<HttpResponse> {
    id.and_then(|u| Some(HttpResponseBuilder::new(StatusCode::OK).json(User(u.id().unwrap()))))
        .ok_or(ServerError::LoginError {
            code: StatusCode::UNAUTHORIZED,
            message: "Not logged in".to_string(),
        })
}

pub async fn auth_middleware(
    req: ServiceRequest,
    next: Next<impl MessageBody + 'static>,
) -> std::result::Result<ServiceResponse<impl MessageBody>, Error> {
    if req.get_identity().is_err() {
        return Ok(req.into_response(
            HttpResponse::Unauthorized()
                .body("not logged in")
                .map_into_right_body(),
        ));
    }

    next.call(req)
        .await
        .map(ServiceResponse::map_into_left_body)
}
