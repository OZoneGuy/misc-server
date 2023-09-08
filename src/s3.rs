use actix_web::{
    get,
    web::{scope, Data, Query, ServiceConfig},
};

use actix_web::web::Json;
use actix_web_lab::middleware::from_fn;
use aws_sdk_s3::Client;
use base64::{engine::general_purpose::STANDARD_NO_PAD, Engine as _};
use serde::{Deserialize, Serialize};

use crate::errors::{Result, ServerError};
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

#[derive(Deserialize, Serialize, Debug)]
enum ObjectType {
    File,
    Dir,
}

#[derive(Deserialize, Serialize, Debug)]
struct ObjectEntry {
    name: String,
    kind: ObjectType,
}
// struct ObjectList(Vec<String>);

#[derive(Deserialize, Serialize, Debug)]
struct S3Object {
    blob: String,
    name: String,
    mime_type: String,
}

#[get("/list_objects")]
async fn list_objects(
    path: Option<Query<S3Query>>,
    s3: Data<Client>,
    config: Data<Config>,
) -> Result<Json<Vec<ObjectEntry>>> {
    let prefix = &path.map(|p| p.path.clone()).unwrap_or("".to_string());

    let objects = s3
        .list_objects_v2()
        .bucket(&config.bucket_name)
        .delimiter("/")
        .prefix(prefix)
        .send()
        .await?
        .contents
        .ok_or(ServerError::ListObjects {
            message: "No contents".to_string(),
        })?
        .into_iter()
        .map(|o| ObjectEntry {
            name: o.key.clone().unwrap(),
            kind: if o.key.unwrap().chars().last().unwrap() == '/' {
                ObjectType::Dir
            } else {
                ObjectType::File
            },
        })
        .collect();
    Ok(Json(objects))
}

#[get("/get_object")]
async fn get_object(
    file_path: Option<Query<S3Query>>,
    s3: Data<Client>,
    config: Data<Config>,
) -> Result<Json<S3Object>> {
    if file_path.is_none() {
        return Err(ServerError::GetObject {
            message: "No file path".to_string(),
        });
    }

    let key = &file_path.unwrap().path;

    let obj = s3
        .get_object()
        .bucket(&config.bucket_name)
        .key(key)
        .send()
        .await?;

    let bytes = obj
        .body
        .collect()
        .await
        .map_err(|e| ServerError::GetObject {
            message: e.to_string(),
        })?;

    let blob = STANDARD_NO_PAD.encode(bytes.to_vec());

    let mime_type = obj.content_type.ok_or(ServerError::GetObject {
        message: "No content type".to_string(),
    })?;

    let name = key.split('/').last().unwrap().to_string();

    Ok(Json(S3Object {
        blob,
        name,
        mime_type,
    }))
}
