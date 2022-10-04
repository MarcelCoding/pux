use std::convert::Infallible;
use std::io;
use std::net::SocketAddr;
use std::sync::Arc;

use hyper::server::conn::Http;
use hyper::service::service_fn;
use tokio::net::{TcpListener, TcpStream};
use tokio_rustls::TlsAcceptor;
use tracing::error;

use crate::config::EntrypointConfig;
use crate::error::PuxResult;
use crate::handler::Handler;
use crate::ServerConfig;

pub(crate) struct Entrypoint {
  id: String,
  listener: TcpListener,
  handler: Arc<Handler>,
  tls_acceptor: Option<Arc<TlsAcceptor>>,
}

impl Entrypoint {
  pub(crate) async fn bind(
    config: &EntrypointConfig,
    handler: Arc<Handler>,
    tls_config: Option<Arc<ServerConfig>>,
  ) -> io::Result<Self> {
    let listener = TcpListener::bind(config.addr).await?;
    let tls_acceptor = tls_config.map(|config| Arc::new(TlsAcceptor::from(config)));

    Ok(Self {
      id: config.id.to_string(),
      listener,
      handler,
      tls_acceptor,
    })
  }

  async fn accept_stram(&self) -> PuxResult<Option<(TcpStream, SocketAddr)>> {
    Ok(Some(self.listener.accept().await?))
  }

  pub(crate) async fn accept(&self) -> PuxResult<()> {
    while let Some((stream, peer_addr)) = self.accept_stram().await? {
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
          tokio::spawn(async move {
            let conn = Http::new()
              .http1_only(true)
              .serve_connection(stream, service);
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

    error!("Entrypoint {} stopped", self.id);
    Ok(())
  }

  pub(crate) fn id(&self) -> &str {
    self.id.as_str()
  }
}
