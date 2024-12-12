use std::env;
use std::net::SocketAddr;
use std::sync::Arc;

use bytes::{Buf, Bytes};
use environment::Environment;
use http_body_util::{BodyExt, Full};
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{body::Incoming as IncomingBody, header, Method, Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use once_cell::sync::Lazy;
use rand::Rng;
use tokio::net::TcpListener;
use tokio::sync::RwLock;
use tokio::time::{sleep, Duration};
use tracing::{error, info, instrument};
use tracing_subscriber;

type GenericError = Box<dyn std::error::Error + Send + Sync>;
type Result<T> = std::result::Result<T, GenericError>;
type BoxBody = http_body_util::combinators::BoxBody<Bytes, hyper::Error>;

const DEFAULT_MIN_DURATION: u64 = 10;
const DEFAULT_MAX_DURATION: u64 = 10;
const DEFAULT_ERROR_RATE: f64 = 0.0;

static GLOBAL_STATE: Lazy<Arc<RwLock<GlobalState>>> =
    Lazy::new(|| Arc::new(RwLock::new(GlobalState::default())));

struct GlobalState {
    min_duration: u64,
    max_duration: u64,
    error_rate: f64,
}

impl Default for GlobalState {
    fn default() -> Self {
        GlobalState {
            min_duration: DEFAULT_MIN_DURATION,
            max_duration: DEFAULT_MAX_DURATION,
            error_rate: DEFAULT_ERROR_RATE,
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let env = Environment::from_env();

    let port = env::var("PORT")
        .unwrap_or_else(|_| "3000".to_string())
        .parse::<u16>()?;
    let addr = match env {
        Environment::Local => SocketAddr::from(([127, 0, 0, 1], port)),
        Environment::DockerCompose => SocketAddr::from(([0, 0, 0, 0], port)),
    };
    let listener = TcpListener::bind(addr).await?;
    info!("Listening on http://{}", addr);

    loop {
        let (stream, _) = listener.accept().await?;
        let io = TokioIo::new(stream);

        tokio::task::spawn(async move {
            let service = service_fn(move |req| router(req));

            if let Err(err) = http1::Builder::new().serve_connection(io, service).await {
                error!("Failed to serve connection: {:?}", err);
            }
        });
    }
}

#[instrument(skip_all)]
async fn router(req: Request<IncomingBody>) -> Result<Response<BoxBody>> {
    info!("Received request: {} {}", req.method(), req.uri().path());

    match (req.method(), req.uri().path()) {
        (&Method::GET, "/health") => {
            let res = health_check().await;
            if let Ok(ref r) = res {
                info!("Response status: {}", r.status());
            }
            res
        }
        (&Method::POST, "/setup") => {
            let res = setup(req).await;
            if let Ok(ref r) = res {
                info!("Response status: {}", r.status());
            }
            res
        }
        (&Method::POST, "/work") => {
            let res = work(req).await;
            if let Ok(ref r) = res {
                info!("Response status: {}", r.status());
            }
            res
        }
        _ => {
            let res = Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(full("Not Found"))
                .unwrap();
            info!("Response status: {}", res.status());
            Ok(res)
        }
    }
}

#[instrument(skip_all)]
async fn health_check() -> Result<Response<BoxBody>> {
    let response = Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "text/plain")
        .body(full("OK"))?;
    Ok(response)
}

#[instrument(skip_all)]
async fn setup(req: Request<IncomingBody>) -> Result<Response<BoxBody>> {
    let whole_body = req.collect().await?.aggregate();
    let data: serde_json::Value = serde_json::from_reader(whole_body.reader())?;

    let min_duration = if let Some(str) = data.get("min_duration").and_then(|v| v.as_str()) {
        str.parse::<u64>().unwrap_or(DEFAULT_MIN_DURATION)
    } else {
        DEFAULT_MIN_DURATION
    };

    let max_duration = if let Some(str) = data.get("max_duration").and_then(|v| v.as_str()) {
        str.parse::<u64>().unwrap_or(min_duration)
    } else {
        min_duration
    }
    .max(DEFAULT_MAX_DURATION);

    let error_rate = if let Some(str) = data.get("error_rate").and_then(|v| v.as_str()) {
        str.parse::<f64>().unwrap_or(DEFAULT_ERROR_RATE)
    } else {
        DEFAULT_ERROR_RATE
    }
    .clamp(0.0, 1.0);

    {
        let mut state = GLOBAL_STATE.write().await;
        state.min_duration = min_duration;
        state.max_duration = max_duration;
        state.error_rate = error_rate;
    }

    let msg = format!(
        "Setup done with min_duration: {}, max_duration: {}, error_rate: {}",
        min_duration, max_duration, error_rate
    );

    info!("{}", msg);

    let response = Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "text/plain")
        .body(full(msg))?;
    Ok(response)
}

#[instrument(skip_all)]
async fn work(req: Request<IncomingBody>) -> Result<Response<BoxBody>> {
    let whole_body = req.collect().await?.aggregate();
    let data: serde_json::Value = serde_json::from_reader(whole_body.reader())?;

    let multiplier = if let Some(duration_str) = data.get("multiplier").and_then(|v| v.as_str()) {
        duration_str.parse::<u64>().unwrap_or(1).clamp(1, 10)
    } else {
        1
    };

    let random_duration = {
        let state = GLOBAL_STATE.read().await;
        rand::thread_rng().gen_range(state.min_duration..=state.max_duration)
    };
    let duration = multiplier * random_duration;

    let status_code = {
        let state = GLOBAL_STATE.read().await;
        if rand::thread_rng().gen_bool(state.error_rate) {
            StatusCode::INTERNAL_SERVER_ERROR
        } else {
            StatusCode::OK
        }
    };

    sleep(Duration::from_millis(duration)).await;

    let response = Response::builder()
        .status(status_code)
        .header(header::CONTENT_TYPE, "text/plain")
        .body(full("Work done"))?;
    Ok(response)
}

fn full<T: Into<Bytes>>(chunk: T) -> BoxBody {
    Full::new(chunk.into())
        .map_err(|never| match never {})
        .boxed()
}
