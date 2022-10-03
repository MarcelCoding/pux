use std::net::{IpAddr, SocketAddr};
use std::time::{Duration, Instant};

use hyper::header::{CONTENT_TYPE, HOST, SERVER};
use hyper::http::HeaderValue;
use hyper::{Body, Request, Response, StatusCode};
use mime::TEXT_HTML_UTF_8;
use tracing::{error, warn};

use crate::error::PuxError::Status;
use crate::routes::Routes;

const ERROR_PAGE: &str = include_str!("error.html");

pub(crate) struct Handler {
  routes: Routes,
}

impl Handler {
  pub(crate) fn new(routes: Routes) -> Self {
    Self { routes }
  }
}

impl Handler {
  pub(crate) async fn handle(&self, peer_addr: SocketAddr, req: Request<Body>) -> Response<Body> {
    let start = Instant::now();

    let host = req
      .headers()
      .get(HOST)
      .and_then(|raw| raw.to_str().ok())
      .map(|with_port| {
        with_port
          .rfind(':')
          .map(|index| &with_port[..index])
          .unwrap_or(with_port)
      })
      .map(|host| host.to_string());

    let service = match host {
      Some(ref host) => {
        let path = req.uri().path().split('/').collect::<Vec<&str>>();
        self.routes.find(host, &path)
      }
      None => None,
    };

    let code = match service {
      None => StatusCode::NOT_FOUND,
      Some(service) => match service.handle(req).await {
        Ok(mut resp) => {
          resp
            .headers_mut()
            .insert(SERVER, HeaderValue::from_static("pux"));
          return resp;
        }
        Err(Status(code)) => code,
        Err(err) => {
          warn!("Handled error while handling request: {}", err);
          StatusCode::INTERNAL_SERVER_ERROR
        }
      },
    };

    let elapsed = start.elapsed();

    error_page(
      code,
      peer_addr.ip(),
      host.unwrap_or_else(|| "unknown".to_string()),
      elapsed,
    )
  }
}

fn error_page(
  code: StatusCode,
  peer_addr: IpAddr,
  host: String,
  elapsed: Duration,
) -> Response<Body> {
  let page = ERROR_PAGE
    .replace("{{CODE}}", code.as_str())
    .replace("{{REASON}}", code.canonical_reason().unwrap_or(""))
    .replace("{{PEER_ADDR}}", &format!("{}", peer_addr))
    .replace("{{HOST}}", &host)
    .replace("{{ELAPSED}}", &format!("{:?}", elapsed));

  let result = Response::builder()
    .status(code)
    .header(CONTENT_TYPE, TEXT_HTML_UTF_8.as_ref())
    .header(SERVER, "pux")
    .body(page.into());

  match result {
    Ok(resp) => resp,
    Err(err) => {
      error!("Fatal error while creating error page: {}", err);
      let mut response = Response::new("Fatal Error".into());
      *response.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
      response
    }
  }
}
