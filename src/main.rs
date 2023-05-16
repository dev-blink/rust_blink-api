use axum::{
    routing::get,
    response::Json,
    Router, extract::{Path, State}, http::StatusCode,
};

use serde_json::{Value, json};
use cloud_storage::{Client, Bucket};
use tokio::time::Instant;
use std::{sync::Arc, io::Cursor};
use image::{io::Reader as ImgReader, ImageOutputFormat};
use rayon::prelude::*;

struct AppState {
    client:Client,
    bucket: Bucket,
}

const BUCKET_NAME: &'static str = "cdn.blinkbot.me";

#[tokio::main]
async fn main() {
   
    println!("Starting :D");

    let client: Client = Client::new();
    let bucket = client.bucket().read(BUCKET_NAME).await.expect("unable to connect to cdn");

    let state: Arc<AppState> = Arc::new(AppState {
        client,
        bucket,
    });
    let app: Router = Router::new()
        .route("/", get(route_index))
        .route("/social/images/ship/:colour", get(route_ship))
        .with_state(state);

    axum::Server::bind(&"0.0.0.0:80".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn route_index() -> (StatusCode, Json<Value>) {
    ok(json!({
        "message":"hello world"
    }))
}

async fn route_ship(Path(raw_colour): Path<String>, State(state): State<Arc<AppState>>) -> (StatusCode, Json<Value>) {

    let before_req = Instant::now();


    // Input validation
    let colour: u32 = match raw_colour.parse() {
        Ok(c) => c,
        Err(_) => {
            return err("unable to parse colour as u32", StatusCode::BAD_REQUEST)
        }
    };

    if colour > 0xFFFFFF {
        return err("colour out of bounds (greater than 0xFFFFFF", StatusCode::BAD_GATEWAY);
    }


    let uri: String = format!("assets/ships/{}.png", colour);

    // if it already exists return bucket link
    match state.client.object().read(BUCKET_NAME, &uri).await {
        Ok(_) => {
            return ok(json!({
                "url": fqd(uri),
                "new": false
            }))
        },
        Err(_) => {},
    }


    let default: Vec<u8> = if let Ok(buf) = state.client.object().download(BUCKET_NAME, "assets/ship.png").await {
        buf
    } else {
        return err("cdn currently unavailable", StatusCode::INTERNAL_SERVER_ERROR);
    };

    let mut buf: Vec<u8> = Vec::with_capacity(default.len());

    let mut img: image::DynamicImage = ImgReader::new(Cursor::new(default)).with_guessed_format().unwrap().decode().unwrap();

    let recolour: [u8; 3] = [
        ((colour & 0xFF0000) >> 16) as u8,
        ((colour & 0x00FF00) >> 8) as u8,
        (colour & 0x0000FF) as u8,
    ];


    
    let before_modify = Instant::now();


    img = tokio::task::spawn_blocking(move || {
        img.as_mut_rgba8().unwrap().as_raw_mut().par_chunks_mut(4).for_each(|px| {
            if px[3] != 0x00 {
                px[0..3].clone_from_slice(&recolour);
            }
        });
        img
    }).await.unwrap();

    let modify_time: std::time::Duration = Instant::now() - before_modify;

    img.write_to(&mut Cursor::new(&mut buf), ImageOutputFormat::Png).unwrap();

    state.client.object().create(BUCKET_NAME, buf, &uri, "image/png").await.unwrap();

    let req_time = Instant::now() - before_req;

    ok(json!({
        "new":true,
        "url": fqd(uri),
        "modify_time": modify_time.as_secs_f32(),
        "req_time": req_time.as_secs_f32(),
    }))
}

fn response(success: bool, code: StatusCode, data: Value) -> (StatusCode, Json<Value>) {
    (
        code,
        Json(json!({
            "success": success,
            "data": data,
        }))
    )
}

fn ok(data: Value) -> (StatusCode, Json<Value>) {
    response(true, StatusCode::OK, data)
}

fn err(message: &str, code: StatusCode) -> (StatusCode, Json<Value>) {
    response(false, code, json!({"message":message}))
}

fn fqd(uri: String) -> String {
    format!("https://{}/{}", BUCKET_NAME, uri)
}