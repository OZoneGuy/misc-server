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
use ldap3::{Ldap, LdapConnAsync};
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

pub async fn create_ldap_conn(url: &str) -> Result<(LdapConnAsync, Ldap)> {
    let (con, ldap) = LdapConnAsync::new(url)
        .await
        .map_err(|e| ServerError::LoginError {
            code: StatusCode::INTERNAL_SERVER_ERROR,
            message: format!("Failed to connect to LDAP server: {}", e.to_string()),
        })?;

    Ok((con, ldap))
}

#[post("/login")]
pub async fn login(req: HttpRequest, login_details: Json<LoginRequest>) -> Result<HttpResponse> {
    // login to ldap server and get user_id
    let (con, mut ldap) = create_ldap_conn("ldap://localhost:3890").await?;

    ldap3::drive!(con);

    let bound = ldap
        .simple_bind(
            &format!(
                "uid={},ou=people,dc=omaralkersh,dc=com",
                login_details.username
            ),
            &login_details.password,
        )
        .await
        .map_err(|e| ServerError::LoginError {
            code: StatusCode::UNAUTHORIZED,
            message: e.to_string(),
        })?;

    if bound.rc != 0 {
        return Err(ServerError::LoginError {
            // TODO: get better status codes
            code: StatusCode::INTERNAL_SERVER_ERROR,
            message: format!("Failed to login. Bind returned non-zero: {}", bound.rc),
        });
    };

    Identity::login(&req.extensions(), login_details.username.clone()).map_err(|e| {
        ServerError::LoginError {
            code: StatusCode::INTERNAL_SERVER_ERROR,
            message: format!("Failed to save session: {}", e.to_string()),
        }
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
