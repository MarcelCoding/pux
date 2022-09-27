use std::fmt::{Debug, Formatter};
use std::io;

pub(crate) enum Error {
  Connect(io::Error),
  Tls(io::Error),
  HttpHandshake(hyper::Error),
  Other(io::Error),
  Forward(hyper::Error),
}

impl Debug for Error {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::Connect(inner) => write!(f, "Error while connecting: {:?}", inner),
      Self::Tls(inner) => write!(f, "Error while doing tls handshake: {:?}", inner),
      Self::HttpHandshake(inner) => write!(f, "Error while doing http initialization: {:?}", inner),
      Self::Other(inner) => write!(f, "Unknown error: {:?}", inner),
      Self::Forward(inner) => write!(f, "Unable to forward request: {:?}", inner),
    }
  }
}
