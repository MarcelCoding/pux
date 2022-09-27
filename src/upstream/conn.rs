use std::io;
use std::io::IoSlice;
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use hyper::client::conn::Builder;
use hyper::client::conn::SendRequest;
use hyper::{Body, Request, Response};
use once_cell::sync::Lazy;
use pin_project::pin_project;
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio::net::TcpStream;
use tokio_rustls::client::TlsStream;
use tokio_rustls::rustls::{ClientConfig, OwnedTrustAnchor, RootCertStore, ServerName};
use tokio_rustls::TlsConnector;
use tracing::error;

use crate::upstream::error::Error;

static TLS_CONNECTOR: Lazy<TlsConnector> = Lazy::new(|| {
  let mut cert_store = RootCertStore::empty();
  cert_store.add_server_trust_anchors(webpki_roots::TLS_SERVER_ROOTS.0.iter().map(|a| {
    OwnedTrustAnchor::from_subject_spki_name_constraints(a.subject, a.spki, a.name_constraints)
  }));

  let config = ClientConfig::builder()
    .with_safe_defaults()
    .with_root_certificates(cert_store)
    .with_no_client_auth();

  TlsConnector::from(Arc::new(config))
});

#[pin_project(project = ConnectionProj)]
enum Connection {
  Raw(#[pin] TcpStream),
  Tls(#[pin] TlsStream<TcpStream>),
}

pub(crate) struct HttpConnection {
  send: SendRequest<Body>,
}

impl Connection {
  pub(crate) async fn open(
    addr: &SocketAddr,
    sni: &Option<ServerName>,
  ) -> Result<Connection, Error> {
    let stream = match TcpStream::connect(addr).await {
      Ok(stream) => stream,
      Err(err) => return Err(Error::Connect(err)),
    };

    if let Err(err) = stream.set_nodelay(true) {
      return Err(Error::Other(err));
    }

    match sni {
      None => Ok(Self::Raw(stream)),
      Some(name) => match TLS_CONNECTOR.connect(name.clone(), stream).await {
        Ok(tls_stream) => Ok(Self::Tls(tls_stream)),
        Err(err) => Err(Error::Tls(err)),
      },
    }
  }
}

impl HttpConnection {
  pub(crate) async fn open(addr: &SocketAddr, sni: &Option<ServerName>) -> Result<Self, Error> {
    let conn = Connection::open(addr, sni).await?;

    let (send, conn) = match Builder::new().handshake(conn).await {
      Ok(data) => data,
      Err(err) => return Err(Error::HttpHandshake(err)),
    };

    tokio::spawn(async move {
      if let Err(err) = conn.await {
        error!("Error while maintaining connection: {}", err);
      }
    });

    Ok(Self { send })
  }

  pub(crate) async fn send(&mut self, req: Request<Body>) -> Result<Response<Body>, Error> {
    match self.send.send_request(req).await {
      Ok(resp) => Ok(resp),
      Err(err) => Err(Error::Forward(err)),
    }
  }
}

impl AsyncRead for Connection {
  fn poll_read(
    self: Pin<&mut Self>,
    cx: &mut Context<'_>,
    buf: &mut ReadBuf<'_>,
  ) -> Poll<io::Result<()>> {
    match self.project() {
      ConnectionProj::Raw(stream) => stream.poll_read(cx, buf),
      ConnectionProj::Tls(stream) => stream.poll_read(cx, buf),
    }
  }
}

impl AsyncWrite for Connection {
  fn poll_write(
    self: Pin<&mut Self>,
    cx: &mut Context<'_>,
    buf: &[u8],
  ) -> Poll<Result<usize, io::Error>> {
    match self.project() {
      ConnectionProj::Raw(stream) => stream.poll_write(cx, buf),
      ConnectionProj::Tls(stream) => stream.poll_write(cx, buf),
    }
  }

  fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), io::Error>> {
    match self.project() {
      ConnectionProj::Raw(stream) => stream.poll_flush(cx),
      ConnectionProj::Tls(stream) => stream.poll_flush(cx),
    }
  }

  fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), io::Error>> {
    match self.project() {
      ConnectionProj::Raw(stream) => stream.poll_shutdown(cx),
      ConnectionProj::Tls(stream) => stream.poll_shutdown(cx),
    }
  }

  fn poll_write_vectored(
    self: Pin<&mut Self>,
    cx: &mut Context<'_>,
    bufs: &[IoSlice<'_>],
  ) -> Poll<Result<usize, io::Error>> {
    match self.project() {
      ConnectionProj::Raw(stream) => stream.poll_write_vectored(cx, bufs),
      ConnectionProj::Tls(stream) => stream.poll_write_vectored(cx, bufs),
    }
  }

  fn is_write_vectored(&self) -> bool {
    match self {
      Connection::Raw(stream) => stream.is_write_vectored(),
      Connection::Tls(stream) => stream.is_write_vectored(),
    }
  }
}
