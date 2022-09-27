use std::convert::Infallible;
use std::io;
use std::sync::Arc;

use hyper::server::conn::Http;
use hyper::service::service_fn;
use tokio::net::TcpListener;
use tokio::select;
use tokio::sync::broadcast::Receiver;
use tokio_rustls::TlsAcceptor;
use tracing::error;

use crate::config::EntrypointConfig;
use crate::error::PuxResult;
use crate::handler::Handler;
use crate::ServerConfig;

pub struct Entrypoint {
  id: String,
  listener: TcpListener,
  handler: Arc<Handler>,
  tls_acceptor: Option<Arc<TlsAcceptor>>,
}

impl Entrypoint {
  pub async fn bind(
    config: &EntrypointConfig,
    handler: Arc<Handler>,
    tls_config: Option<Arc<ServerConfig>>,
  ) -> io::Result<Self> {
    Ok(Self {
      id: config.id.to_string(),
      listener: TcpListener::bind(config.addr).await?,
      handler,
      tls_acceptor: tls_config.map(|config| Arc::new(TlsAcceptor::from(config))),
    })
  }

  pub async fn accept(&self, mut shutdown: Receiver<()>) -> PuxResult<()> {
    loop {
      let (stream, peer_addr) = select! {
       resp = self.listener.accept() => resp?,
        _ = shutdown.recv() => return Ok(()),
      };

      stream.set_nodelay(true)?;

      let service = {
        let handler = self.handler.clone();
        service_fn(move |req| {
          let handler = handler.clone();

          async move { Ok::<_, Infallible>(handler.handle(peer_addr, req).await) }
        })
      };

      match &self.tls_acceptor {
        None => {
          let conn = Http::new().serve_connection(stream, service);
          tokio::spawn(async move {
            if let Err(err) = conn.await {
              error!("Failed to serve connection: {}", err);
            }
          });
        }
        Some(tls_acceptor) => {
          let tls_acceptor = tls_acceptor.clone();
          tokio::spawn(async move {
            let tls_stream = match tls_acceptor.accept(stream).await {
              Ok(tls_stream) => tls_stream,
              Err(err) => {
                error!("Error while tls handshake: {}", err);
                return;
              }
            };

            let conn = Http::new().serve_connection(tls_stream, service);

            if let Err(err) = conn.await {
              error!("Failed to serve connection: {}", err);
            }
          });
        }
      }
    }
  }

  pub fn id(&self) -> &str {
    &self.id
  }
}
