use std::pin::Pin;

use actix_identity::{Identity, IdentityExt};
use actix_web::{
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    get, post,
    web::ServiceConfig,
    Error, HttpMessage, HttpRequest, HttpResponse, Responder,
};
use futures::{
    future::{ok, Ready},
    Future,
};

pub fn auth_config(cfg: &mut ServiceConfig) {
    cfg.service(login).service(logout).service(user);
}

#[post("/login")]
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

pub struct AuthMiddleware;
impl<S, B> Transform<S, ServiceRequest> for AuthMiddleware
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = AuthMiddlewareService<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ok(AuthMiddlewareService { service })
    }
}

pub struct AuthMiddlewareService<S> {
    service: S,
}

impl<S, B> Service<ServiceRequest> for AuthMiddlewareService<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>>>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        if req.get_identity().is_err() {
            // req.into_response(res)
            // let resp: ServiceResponse<B> = ServiceResponse::new(
            return Box::pin(async move {
                Ok(req.into_response(HttpResponse::Unauthorized().finish().into_body()))
            });
        };

        return Box::pin(async move {
            let res = self.service.call(req).await?;
            Ok(res)
        });
    }
}
