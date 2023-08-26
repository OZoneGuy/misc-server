use serde::Serialize;
use thiserror::Error;

use aws_sdk_s3::{error::SdkError, operation::list_objects_v2::ListObjectsV2Error};

#[derive(Serialize, Debug, Error)]
pub enum ServerError {
    #[error("Failed the health check with the following errors: {errors:?}")]
    HealthCheckError { errors: Vec<String> },
    #[error("Failed to retrieve objects list: {message}")]
    ListObjectsError { message: String },
}

impl From<SdkError<ListObjectsV2Error>> for ServerError {
    fn from(e: SdkError<ListObjectsV2Error>) -> Self {
        ServerError::ListObjectsError {
            message: e.to_string(),
        }
    }
}

pub type Result<T> = std::result::Result<T, ServerError>;

impl actix_web::error::ResponseError for ServerError {
    fn error_response(&self) -> actix_web::HttpResponse {
        match self {
            ServerError::HealthCheckError { .. } => {
                actix_web::HttpResponse::InternalServerError().json(self)
            }
            ServerError::ListObjectsError { .. } => {
                actix_web::HttpResponse::InternalServerError().json(self)
            }
        }
    }
}
