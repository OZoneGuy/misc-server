use actix_identity::{Identity, IdentityExt};
use actix_web::{
    body::MessageBody,
    dev::{ServiceRequest, ServiceResponse},
    get, post,
    web::ServiceConfig,
    Error, HttpMessage, HttpRequest, HttpResponse, Responder,
};
use actix_web_lab::middleware::Next;

pub fn auth_config(cfg: &mut ServiceConfig) {
    cfg.service(login).service(logout).service(user);
}

#[get("/login")]
pub async fn login(req: HttpRequest) -> impl Responder {
    Identity::login(&req.extensions(), "user_id".to_string()).unwrap();

    HttpResponse::Ok().body("Logged in")
}

#[post("/logout")]
pub async fn logout(id: Identity) -> impl Responder {
    id.logout();

    HttpResponse::Ok()
}

#[get("/user")]
pub async fn user(id: Option<Identity>) -> impl Responder {
    if let Some(id) = id {
        HttpResponse::Ok().body(format!("User: {}", id.id().unwrap()))
    } else {
        HttpResponse::Unauthorized().body("Not logged in")
    }
}

pub async fn auth_middleware(
    req: ServiceRequest,
    next: Next<impl MessageBody + 'static>,
) -> Result<ServiceResponse<impl MessageBody>, Error> {
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
