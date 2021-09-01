use std::{convert::Infallible, net::SocketAddr, sync::Arc, time::Duration};

use futures::TryFutureExt;
use http::{Method, Request, Response, StatusCode};
use hyper::server::conn::AddrStream;
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Server};
use tokio::{join, signal};

use monitor_rust::feed;

pub struct Handler {
    listener_port: u16,
    request_timeout: Duration,
    observer_url: String,
}

impl Handler {
    pub fn new(listener_port: u16, request_timeout: Duration, observer_url: String) -> Self {
        println!(
            "creating handler with listener port {}, request timeout {:?}",
            listener_port, request_timeout,
        );

        Self {
            listener_port,
            request_timeout,
            observer_url,
        }
    }

    pub async fn start(self) -> Result<(), Box<dyn std::error::Error>> {
        println!("starting server");

        let (feeder, stop_rx) = feed::Feeder::new(self.request_timeout, self.observer_url);
        println!("started feeder");
        let feeder = Arc::new(feeder);

        let addr = ([0, 0, 0, 0], self.listener_port).into();
        let svc = make_service_fn(|socket: &AddrStream| {
            let remote_addr = socket.remote_addr();
            let feeder = feeder.clone();
            async move {
                Ok::<_, Infallible>(service_fn(move |req: Request<Body>| {
                    handle_request(remote_addr, req, feeder.clone()).or_else(
                        |(status, body)| async move {
                            println!("{}", body);
                            Ok::<_, Infallible>(
                                Response::builder()
                                    .status(status)
                                    .body(Body::from(body))
                                    .unwrap(),
                            )
                        },
                    )
                }))
            }
        });

        let poller = feeder.poll(stop_rx);
        let server = Server::try_bind(&addr)?
            .serve(svc)
            .with_graceful_shutdown(handle_sigint());

        // TODO: find a way to signal poll loops
        // call "stop" on sigint
        println!("listener start http://{}", addr);
        let (poller_rs, server_rs) = join!(poller, server);
        match poller_rs {
            Ok(_) => {}
            Err(e) => println!("poll error: {}", e),
        }
        match server_rs {
            Ok(_) => {}
            Err(e) => println!("server error: {}", e),
        }
        println!("listener done http://{}", addr);

        match feeder.stop().await {
            Ok(_) => println!("stopped feeder"),
            Err(e) => println!("failed to stop feeder {}", e),
        }

        Ok(())
    }
}

async fn handle_request(
    addr: SocketAddr,
    req: Request<Body>,
    feeder: Arc<feed::Feeder>,
) -> Result<Response<Body>, (http::StatusCode, String)> {
    let http_version = req.version();
    let method = req.method().clone();
    let cloned_uri = req.uri().clone();
    let path = cloned_uri.path();
    println!(
        "version {:?}, method {}, uri path {}, remote addr {}",
        http_version, method, path, addr,
    );

    let resp = match method {
        Method::GET => {
            let prices = feeder.prices().map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("failed to serde_json::from_str {}", e),
                )
            })?;
            let rs = serde_json::to_string(&prices).unwrap();
            Response::new(Body::from(String::from(rs)))
        }
        _ => Err((
            StatusCode::NOT_FOUND,
            format!("unknown method {} and path {}", method, req.uri().path()),
        ))?,
    };

    Ok(resp)
}

async fn handle_sigint() {
    signal::ctrl_c()
        .await
        .expect("failed to install CTRL+C signal handler");
}
