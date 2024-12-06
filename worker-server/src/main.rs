use std::env;
use std::net::SocketAddr;

use bytes::{Buf, Bytes};
use http_body_util::{BodyExt, Full};
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{body::Incoming as IncomingBody, header, Method, Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use tokio::net::TcpListener;
use tokio::time::{sleep, Duration};
use tracing::{error, info, instrument};
use tracing_subscriber;

type GenericError = Box<dyn std::error::Error + Send + Sync>;
type Result<T> = std::result::Result<T, GenericError>;
type BoxBody = http_body_util::combinators::BoxBody<Bytes, hyper::Error>;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let port = env::var("PORT")
        .unwrap_or_else(|_| "3000".to_string())
        .parse::<u16>()?;
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
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
async fn work(req: Request<IncomingBody>) -> Result<Response<BoxBody>> {
    let whole_body = req.collect().await?.aggregate();
    let data: serde_json::Value = serde_json::from_reader(whole_body.reader())?;

    let duration = if let Some(duration_str) = data.get("duration").and_then(|v| v.as_str()) {
        duration_str.parse::<u64>().unwrap_or(10)
    } else {
        10
    };

    let status_code = if let Some(result_str) = data.get("status_code").and_then(|v| v.as_str()) {
        result_str.parse::<StatusCode>().unwrap_or(StatusCode::OK)
    } else {
        StatusCode::OK
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
