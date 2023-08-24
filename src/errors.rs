use thiserror::Error;

#[derive(Debug, Error)]
pub enum ServerError {
    #[error("Failed the health check with the following errors: {0:?}")]
    HealthCheckError(Vec<String>),
}

pub type Result<T> = std::result::Result<T, ServerError>;

impl actix_web::error::ResponseError for ServerError {
    fn error_response(&self) -> actix_web::HttpResponse {
        match self {
            ServerError::HealthCheckError(errors) => {
                actix_web::HttpResponse::InternalServerError().json(errors)
            }
        }
    }
}
