use axum::{
    routing::get,
    response::{Json, Response},
    extract::TypedHeader,
    headers::authorization::{Authorization, Basic, Credentials},
    Router, extract::{Path, State}, http::{StatusCode, Request}, middleware::Next,
};

use serde_json::{Value, json};
use cloud_storage::{Client, Bucket};
use tokio::time::Instant;
use std::{sync::Arc, io::Cursor};
use image::{io::Reader as ImgReader, ImageOutputFormat};
use rayon::prelude::*;
use rand::{thread_rng, seq::SliceRandom};

const KISS_GIF: [&str; 36] = ["04f82fbf-2b3c-4bbf-a6c5-1c64488f0326.gif", "08f83dc7-8e3c-438d-a7fb-5fb862da705b.gif", "119f9c5e-3c35-4908-b821-c46fbf4301c4.gif", "165dbae0-5b22-4f91-898c-95be50b2dc28.gif", "1cb559c9-82e9-4167-b233-a2e87c08ff73.gif", "1d51a755-464b-47cf-8a21-e8f5a804464d.gif", "1fb24bae-0739-4b3a-8728-e743ede30e09.gif", "214606b1-6576-4533-920d-3c8ab3796c61.gif", "21894266-2c25-4347-9c8a-56dffb6ee073.gif", "2e1a3644-7f2a-40e2-9630-fc3bdcf45b3c.gif", "3e412e66-ec6b-4904-ac31-4b8d89b8b314.gif", "3eac373b-19fb-41c1-80b5-027708ae4850.gif", "40070c3c-d8c3-4463-9d7c-d7185613aaf5.gif", "5f58e6c4-c060-4996-a190-fd5525bef77b.gif", "6c3cf945-51b6-47de-913d-d161540d4bc4.gif", "6e2a642b-3a4d-4fed-8eb2-a1d0d4c4088e.gif", "87dc9790-0ec9-4941-b9c0-b541de4b1d97.gif", "93ad07f7-d643-4b81-8bdd-59f69ca07442.gif", "96123d6e-affa-4c58-acd3-b4cebdc01507.gif", "98001cd7-9d73-4f60-9be5-a515438a09bf.gif", "9d597c75-72cb-4b8f-a40b-3c1065686ee3.gif", "9e396db0-c31b-4bf3-a2b6-c322ae30cb3e.gif", "a5522570-1ac3-4d3a-adfd-d7581285e951.gif", "a7809321-a1dc-4e14-be75-52e8c55862cc.gif", "a7cc7a94-82a9-47ff-8f56-ee7987851e9e.gif", "a9e73c69-7587-4f4c-b4d2-e4758bea0fb7.gif", "ad3578ef-12b1-4c39-9d5f-c53e4c00e0cf.gif", "b9161c71-7755-4b49-aac9-2280e7efcf61.gif", "bfe623a9-47ae-41aa-a064-7ea950477b05.gif", "c5f65acc-a4a9-40c3-8906-6ad66bd88580.gif", "dabb3834-7edf-4cd7-bc70-199cdb4ac805.gif", "df9e40eb-6d23-4d1e-afb2-a48d306ef4df.gif", "e8b38995-e25b-4501-b6e1-3324c05875d8.gif", "ec3076bc-786d-4aed-a21a-2d11f443c54b.gif", "ef7dcf07-ad87-45a2-8efa-d1d9306fea64.gif", "fa036405-3179-413a-a698-dadc49035352.gif"];
const HUG_GIF: [&str; 38] = ["0423dad9-6128-4a28-9517-26533ec6e6b9.gif", "09a2162e-5ed3-4b55-8317-4f95d23b9a05.gif", "0d30ad5c-eac5-4caa-97d1-f6f9d7615872.gif", "11c7b238-0e19-4987-83e4-8d0c7a1eec9f.gif", "11fa7caa-6438-436b-9fab-5d4305b4ef5a.gif", "1652ad7f-8345-4b8e-8e8c-39001f5755d5.gif", "25b1453c-8038-4457-92f9-8ca9924a6520.gif", "2e02fd21-d1af-47c0-8a7d-ba7ac9c1a479.gif", "3d1d4645-f954-4a9b-b274-0f2152df3009.gif", "4de83de8-f2b3-49c8-b026-5c087758a07e.gif", "4f5f1160-93a7-41d8-a3ed-9efdd88add42.gif", "50fc39d6-9c9f-48a9-8635-7b2a027d9a60.gif", "530df111-e60c-4249-9d27-51ff2824244e.gif", "632db834-d4a8-4b38-846d-7c27006a32bf.gif", "724244ab-3b73-4436-aca6-a3d0aae6ce21.gif", "74870528-f76e-4352-8726-96c23abe1248.gif", "7da5de98-9bd2-41e1-b3a3-14087181e23b.gif", "808d925f-89db-4989-93ee-77bb51300350.gif", "82914c62-053e-4c97-bc5a-d0717105f691.gif", "83f1dc55-4374-4ccf-9265-ac4eb85a69aa.gif", "874c639e-5f3b-4ef6-81a7-2375ffc188bc.gif", "8df1967a-35a3-4b79-be11-ffc21831dcc1.gif", "9499e75c-ecb5-4361-a813-b30031949d60.gif", "9d510835-f694-4b33-81b7-d9262c781a93.gif", "9f33c977-79f3-45a5-8660-7ba9bc5d7a70.gif", "a4568fff-28be-4f6e-ac62-5cdb193265e3.gif", "ad2cebee-73c7-4ecb-93ec-3b664e28a0e0.gif", "b391b41c-fabb-4ae6-b8bb-184f23ff76c0.gif", "c2268d19-662d-4d7b-9d15-6325af0a2bc9.gif", "c8a258a6-c34a-4cb3-8690-53666d751434.gif", "ca27a17d-2009-453e-96c6-47677e20282b.gif", "d01ff754-2948-4e7a-8d94-1f8a55f472ec.gif", "e0772455-82e3-4f60-baa1-87d4552e1467.gif", "e2bd9dda-bcb2-4ae3-b02b-4a54cf22027c.gif", "e312130d-c6b0-4817-9fd1-15168ce09109.gif", "e842e684-2a95-4351-bdd1-6e6de29fa03d.gif", "f7f63ba0-8857-47f7-8d5d-5a9a853a3e55.gif", "fc22710e-4754-44b2-8e31-712c05c65bef.gif"];

const API_TOKEN: &str = include_str!("../TOKEN");



struct AppState {
    client:Client,
    bucket: Bucket,
}

const BUCKET_NAME: &'static str = "cdn.blinkbot.me";

struct RawToken {
    token: String
}

impl Credentials for RawToken {
    const SCHEME: &'static str = "Grant";

    fn decode(value: &axum::http::HeaderValue) -> Option<Self> {
        if let Ok(t) = value.to_str() {
            Some(Self {token: t.to_string()})
        } else {
            None
        }
    }

    fn encode(&self) -> axum::http::HeaderValue {
        todo!()
    }
}

async fn check_auth<B>(
    TypedHeader(auth): TypedHeader<Authorization<RawToken>>,
    request: Request<B>,
    next: Next<B>,
) -> Result<Response, (StatusCode, Json<Value>)> {
    if auth.0.token == API_TOKEN {
        Ok(next.run(request).await)
    } else {
        println!("Failed auth with '{}'", auth.0.token);
        Err(err("bad authentication", StatusCode::UNAVAILABLE_FOR_LEGAL_REASONS))
    }
}


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
        .route("/social/images/hug", get(route_hug))
        .route("/social/images/kiss", get(route_kiss))
        .route_layer(axum::middleware::from_fn(check_auth))
        .with_state(state);

    axum::Server::bind(&"0.0.0.0:9001".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn route_index() -> (StatusCode, Json<Value>) {
    ok(json!({
        "message":"hello world"
    }))
}

async fn route_hug() -> (StatusCode, Json<Value>) {
    ok(json!({
        "url": fqd(format!("assets/hugs/{}", HUG_GIF.choose(&mut thread_rng()).unwrap()))
    }))
}


async fn route_kiss() -> (StatusCode, Json<Value>) {
    ok(json!({
        "url": fqd(format!("assets/kisses/{}", KISS_GIF.choose(&mut thread_rng()).unwrap()))
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