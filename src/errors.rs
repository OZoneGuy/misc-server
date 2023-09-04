use actix_web::http::StatusCode;
use serde::Serialize;
use thiserror::Error;

use aws_sdk_s3::{
    error::SdkError,
    operation::{get_object::GetObjectError, list_objects_v2::ListObjectsV2Error},
};

#[derive(Serialize, Debug, Error)]
pub enum ServerError {
    #[error("Failed the health check with the following errors: {errors:?}")]
    HealthCheck { errors: Vec<String> },
    #[error("Failed to retrieve objects list: {message}")]
    ListObjects { message: String },
    #[error("Failed to get object: {message}")]
    GetObject { message: String },
    #[error("Login failed: {message}")]
    Login {
        #[serde(skip_serializing)]
        code: StatusCode,
        message: String,
    },
}

impl From<SdkError<ListObjectsV2Error>> for ServerError {
    fn from(e: SdkError<ListObjectsV2Error>) -> Self {
        ServerError::ListObjects {
            message: e.to_string(),
        }
    }
}

impl From<SdkError<GetObjectError>> for ServerError {
    fn from(e: SdkError<GetObjectError>) -> Self {
        ServerError::GetObject {
            message: e.to_string(),
        }
    }
}

pub type Result<T> = std::result::Result<T, ServerError>;

impl actix_web::error::ResponseError for ServerError {
    fn error_response(&self) -> actix_web::HttpResponse {
        match self {
            ServerError::HealthCheck { .. } => {
                actix_web::HttpResponse::InternalServerError().json(self)
            }
            ServerError::ListObjects { .. } => {
                actix_web::HttpResponse::InternalServerError().json(self)
            }
            ServerError::GetObject { .. } => {
                actix_web::HttpResponse::InternalServerError().json(self)
            }
            ServerError::Login { code, .. } => actix_web::HttpResponse::build(*code).json(self),
        }
    }
}
