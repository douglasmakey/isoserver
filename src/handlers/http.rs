use bytes::Bytes;
use http_body_util::{combinators::BoxBody, BodyExt, Empty, Full};
use hyper::{
    body::{Body, Frame},
    server::conn::http1,
    service::service_fn,
    Method, Request, Response, StatusCode,
};
use hyper_util::rt::TokioIo;
use log::info;
use nix::unistd::gethostname;
use serde::{Deserialize, Serialize};
use std::io;
use tokio::net::TcpListener;

#[derive(Serialize, Deserialize)]
pub struct Whoami {
    pub hostname: String,
}

pub struct HTTPServer {
    address: String,
}

async fn service(
    req: Request<hyper::body::Incoming>,
) -> Result<Response<BoxBody<Bytes, hyper::Error>>, hyper::Error> {
    match (req.method(), req.uri().path()) {
        (&Method::GET, "/whoami") => {
            let hostname = gethostname().expect("Failed getting hostname");
            let hostname = hostname.into_string().expect("Hostname wasn't valid UTF-8");
            let whoami = Whoami { hostname };
            match serde_json::to_string(&whoami) {
                Ok(json) => {
                    let body = Full::new(json.into())
                        .map_err(|never| match never {})
                        .boxed();

                    Ok(Response::new(body))
                }
                Err(_) => Ok(Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .body(empty())
                    .unwrap()),
            }
        }
        (&Method::POST, "/echo") => Ok(Response::new(req.into_body().boxed())),
        _ => Ok(Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(empty())
            .unwrap()),
    }
}

fn empty() -> BoxBody<Bytes, hyper::Error> {
    Empty::<Bytes>::new()
        .map_err(|never| match never {})
        .boxed()
}

impl HTTPServer {
    pub fn new(address: String) -> Self {
        Self { address }
    }

    pub async fn run(self) -> Result<(), io::Error> {
        let listener = TcpListener::bind(&self.address).await?;
        info!("HTTP server listening on: {}", self.address);
        loop {
            let (stream, _) = listener.accept().await?;
            let io = TokioIo::new(stream);
            tokio::task::spawn(async move {
                if let Err(err) = http1::Builder::new()
                    .serve_connection(io, service_fn(service))
                    .await
                {
                    println!("Error serving connection: {:?}", err);
                }
            });
        }
    }
}
