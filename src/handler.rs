use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use std::time::{Duration, Instant};

use hyper::header::{CONTENT_TYPE, HOST};
use hyper::http::HeaderValue;
use hyper::{Body, Request, Response, StatusCode};

use crate::error::PuxError::Status;
use crate::upstream::Upstream;

const ERROR_PAGE: &str = include_str!("error.html");
const TEXT_HTML: HeaderValue = HeaderValue::from_static("text/html");

pub struct Handler {
  routes: HashMap<String, Arc<Upstream>>,
}

impl Handler {
  pub fn new(routes: HashMap<String, Arc<Upstream>>) -> Self {
    Self { routes }
  }
}

impl Handler {
  pub async fn handle(&self, peer_addr: SocketAddr, req: Request<Body>) -> Response<Body> {
    let start = Instant::now();

    let host = req
      .headers()
      .get(HOST)
      .and_then(|raw| raw.to_str().ok())
      .and_then(|with_port| with_port.split(':').next())
      .map(|host| host.to_string());

    let upstream = match host {
      Some(ref host) => self.routes.get(host),
      None => None,
    };

    let err = if let Some(upstream) = upstream {
      match upstream.send(req).await {
        Ok(resp) => return resp,
        Err(err) => err,
      }
    } else {
      Status(StatusCode::NOT_FOUND)
    };

    let code = if let Status(code) = err {
      code
    } else {
      StatusCode::INTERNAL_SERVER_ERROR
    };

    let elapsed = start.elapsed();

    ErrorPage {
      code,
      peer_addr: peer_addr.ip(),
      host: host.unwrap_or_else(|| "unknown".to_string()),
      elapsed,
    }
    .into_response()
  }
}

struct ErrorPage {
  code: StatusCode,
  peer_addr: IpAddr,
  host: String,
  elapsed: Duration,
}

impl ErrorPage {
  fn into_response(self) -> Response<Body> {
    let page = ERROR_PAGE
      .replace("{{CODE}}", self.code.as_str())
      .replace("{{REASON}}", self.code.canonical_reason().unwrap_or(""))
      .replace("{{PEER_ADDR}}", &format!("{}", self.peer_addr))
      .replace("{{HOST}}", &self.host)
      .replace("{{ELAPSED}}", &format!("{:?}", self.elapsed));

    let mut response = Response::new(page.into());
    *response.status_mut() = self.code;
    response.headers_mut().insert(CONTENT_TYPE, TEXT_HTML);

    response
  }
}
