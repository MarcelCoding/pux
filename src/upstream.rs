use std::net::SocketAddr;
use std::str::FromStr;

use conn::handshake;
use hyper::client::conn;
use hyper::client::conn::SendRequest;
use hyper::header::HOST;
use hyper::http::HeaderValue;
use hyper::{Body, Request, Response, Uri};
use tokio::net::TcpStream;
use tokio::sync::{Mutex, RwLock};
use tracing::error;

use crate::PuxResult;

pub struct Upstream {
  addrs: Vec<SocketAddr>,
  conns: RwLock<Vec<UpstreamConnection>>,
}

impl Upstream {
  pub fn new(addrs: Vec<SocketAddr>) -> Self {
    Self {
      addrs,
      conns: RwLock::new(Vec::new()),
    }
  }

  pub async fn send(&self, mut req: Request<Body>) -> PuxResult<Response<Body>> {
    let uri = Uri::from_str(&format!(
      "https://www.google.com{}?{}",
      req.uri().path(),
      req.uri().query().unwrap_or("")
    ))
    .unwrap();

    *req.uri_mut() = uri;
    req
      .headers_mut()
      .insert(HOST, HeaderValue::from_static("www.google.com"));

    Ok(self.pooled_send(req).await?)
  }

  async fn pooled_send(&self, req: Request<Body>) -> hyper::Result<Response<Body>> {
    if self.conns.read().await.is_empty() {
      self
        .conns
        .write()
        .await
        .push(UpstreamConnection::open(self.addrs.first().unwrap()).await);
    }

    self.conns.read().await.first().unwrap().send(req).await
  }
}

struct UpstreamConnection {
  sender: Mutex<SendRequest<Body>>,
}

impl UpstreamConnection {
  pub async fn open(addr: &SocketAddr) -> Self {
    let stream = TcpStream::connect(&addr).await.unwrap();
    let (sender, conn) = handshake(stream).await.unwrap();

    tokio::spawn(async move {
      if let Err(err) = conn.await {
        error!("Upstream connection error: {}", err);
      }
    });

    Self {
      sender: Mutex::new(sender),
    }
  }

  pub async fn send(&self, req: Request<Body>) -> hyper::Result<Response<Body>> {
    self.sender.lock().await.send_request(req).await
  }
}
