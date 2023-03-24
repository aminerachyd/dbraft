use std::{collections::HashMap, io};

use axum::{
    extract::Path,
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};

use log::info;
use serde::Deserialize;

use crate::db_connexion::db_connexion::connect_to_database;

#[derive(Deserialize)]
struct PutItem {
    id: String,
    item: String,
}

pub async fn start_endpoint(port: u32) -> io::Result<()> {
    let api = Router::new()
        .route("/", get(root))
        .route("/item/:id", get(get_item))
        .route("/item", post(put_item));

    info!("Endpoint started, listening on port {}", port);
    axum::Server::bind(&format!("0.0.0.0:{}", port).parse().unwrap())
        .serve(api.into_make_service())
        .await
        .unwrap();

    Ok(())
}

async fn root() -> String {
    "This is a REST endpoint to the backend database\nYou can send GET and POST requests to /item"
        .to_owned()
}

async fn get_item(Path(params): Path<HashMap<String, String>>) -> Result<String, StatusCode> {
    if let Some(id) = params.get("id") {
        let result = connect_to_database("0.0.0.0:8888".to_owned())
            .await
            .unwrap()
            .get(id.to_owned())
            .await;

        if let Ok(item) = result {
            Ok(item)
        } else {
            Err(StatusCode::NOT_FOUND)
        }
    } else {
        Err(StatusCode::UNPROCESSABLE_ENTITY)
    }
}

async fn put_item(Json(payload): Json<PutItem>) -> Result<String, StatusCode> {
    let PutItem { id, item } = payload;

    let result = connect_to_database("0.0.0.0:8888".to_owned())
        .await
        .unwrap()
        .put(id, item)
        .await;

    if let Ok(item) = result {
        Ok(item)
    } else {
        Err(StatusCode::INTERNAL_SERVER_ERROR)
    }
}
