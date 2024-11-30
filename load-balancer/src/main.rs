mod balancing_algorithm;
mod load_balancer;
mod server;

use std::env;
use std::net::SocketAddr;
use std::sync::Arc;

use balancing_algorithm::BalancingAlgorithm;
use bytes::{Buf, Bytes};
use http_body_util::{BodyExt, Full};
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::Uri;
use hyper::{body::Incoming as IncomingBody, header, Method, Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use load_balancer::LoadBalancer;
use server::Server;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::RwLock;
use tracing::{error, info, instrument, warn};
use tracing_subscriber;

type GenericError = Box<dyn std::error::Error + Send + Sync>;
type Result<T> = std::result::Result<T, GenericError>;
type BoxBody = http_body_util::combinators::BoxBody<Bytes, hyper::Error>;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let load_balancer = create_load_balancer().unwrap();
    let load_balancer = Arc::new(RwLock::new(load_balancer));

    let port = env::var("PORT")
        .unwrap_or_else(|_| "80".to_string())
        .parse::<u16>()?;
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = TcpListener::bind(addr).await.map_err(|e| e.to_string())?;
    info!("Listening on http://{}", addr);

    loop {
        let (stream, _) = listener.accept().await.map_err(|e| e.to_string())?;
        let io = TokioIo::new(stream);
        let load_balancer_clone = load_balancer.clone();

        tokio::task::spawn(async move {
            let service = service_fn(move |req| handle_request(req, load_balancer_clone.clone()));

            if let Err(err) = http1::Builder::new().serve_connection(io, service).await {
                error!("Failed to serve connection: {:?}", err);
            }
        });
    }
}

fn create_load_balancer() -> Result<LoadBalancer> {
    let servers = vec![
        Server::new("127.0.0.1:3000".to_string())?,
        Server::new("127.0.0.1:3001".to_string())?,
        Server::new("127.0.0.1:3002".to_string())?,
    ];

    let lb = LoadBalancer::new(servers)?;
    Ok(lb)
}

#[instrument(skip_all)]
async fn handle_request(
    req: Request<IncomingBody>,
    lb: Arc<RwLock<LoadBalancer>>,
) -> Result<Response<BoxBody>> {
    info!("Received request: {} {}", req.method(), req.uri().path());
    match (req.method(), req.uri().path()) {
        (&Method::POST, "/algo") => change_algo(req, lb).await,
        _ => forward_request(req, lb).await,
    }
}

#[instrument(skip_all)]
async fn change_algo(
    req: Request<IncomingBody>,
    lb: Arc<RwLock<LoadBalancer>>,
) -> Result<Response<BoxBody>> {
    let whole_body = req.collect().await?.aggregate();
    let data: serde_json::Value = serde_json::from_reader(whole_body.reader())?;
    if let Some(algo_value) = data.get("algo").and_then(|v| v.as_str()) {
        match BalancingAlgorithm::try_from(algo_value) {
            Ok(algo) => {
                {
                    let mut lb = lb.write().await;
                    lb.set_algorithm(algo);
                }

                let msg = format!("Algorithm changed successfully to {}", algo);
                info!(msg);

                let response = Response::builder()
                    .status(StatusCode::OK)
                    .header(header::CONTENT_TYPE, "text/plain")
                    .body(full(msg))?;
                Ok(response)
            }
            Err(_) => {
                let msg = format!("Invalid algorithm value '{}'", algo_value);
                warn!(msg);
                let response = Response::builder()
                    .status(StatusCode::BAD_REQUEST)
                    .header(header::CONTENT_TYPE, "text/plain")
                    .body(full(msg))?;
                Ok(response)
            }
        }
    } else {
        let msg = "Missing or invalid 'algo' key";
        warn!(msg);
        let response = Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .header(header::CONTENT_TYPE, "text/plain")
            .body(full(msg))?;
        Ok(response)
    }
}

#[instrument(skip_all)]
async fn forward_request(
    req: Request<IncomingBody>,
    lb: Arc<RwLock<LoadBalancer>>,
) -> Result<Response<BoxBody>> {
    let worker_addr = {
        let mut lb = lb.write().await;
        let server = lb.next_server();
        server.get_address().to_string()
    };

    let worker_uri_string = format!(
        "http://{}{}",
        worker_addr,
        req.uri()
            .path_and_query()
            .map(|x| x.as_str())
            .unwrap_or("/")
    );

    let worker_uri = worker_uri_string.parse::<Uri>().expect("uri parse");

    let headers = req.headers().clone();

    let mut worker_req = Request::builder()
        .method(req.method())
        .uri(worker_uri)
        .body(req.into_body())
        .expect("request builder");

    for (key, value) in headers.iter() {
        worker_req.headers_mut().insert(key, value.clone());
    }

    let client_stream = TcpStream::connect(&worker_addr).await.unwrap();
    let io = TokioIo::new(client_stream);

    let (mut sender, conn) = hyper::client::conn::http1::handshake(io).await?;
    tokio::task::spawn(async move {
        if let Err(err) = conn.await {
            error!("Connection failed: {:?}", err);
        }
    });

    info!("Forwarding request to {}", worker_addr);

    let worker_res = sender.send_request(worker_req).await?;
    let res_body = worker_res.into_body().boxed();

    {
        let mut lb = lb.write().await;
        let server = lb.get_server_by_address(&worker_addr).unwrap();
        server.decrement_connections();
    }

    Ok(Response::new(res_body))
}

fn full<T: Into<Bytes>>(chunk: T) -> BoxBody {
    Full::new(chunk.into())
        .map_err(|never| match never {})
        .boxed()
}
