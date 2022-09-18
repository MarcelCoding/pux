use fs::File;
use io::BufReader;
use std::collections::HashMap;
use std::sync::Arc;
use std::{fs, io};

use rustls_pemfile::Item;
use tokio_rustls::rustls;
use tokio_rustls::rustls::server::{ClientHello, ResolvesServerCert};
use tokio_rustls::rustls::sign::{any_supported_type, CertifiedKey, SigningKey};
use tokio_rustls::rustls::PrivateKey;
use tokio_rustls::webpki::DnsName;

struct CertData {
  key: Vec<String>,
  chain: Vec<Vec<String>>,
}

pub struct CertStore {
  certs: HashMap<String, Arc<CertifiedKey>>,
  fallback_name: String,
}

impl CertStore {
  pub fn new(fallback_name: DnsName) -> Self {
    let mut name = <DnsName as AsRef<str>>::as_ref(&fallback_name).to_string();
    name.make_ascii_lowercase();

    Self {
      certs: HashMap::new(),
      fallback_name: name,
    }
  }

  pub fn insert(&mut self, name: DnsName, cert: Arc<CertifiedKey>) -> Option<Arc<CertifiedKey>> {
    let mut name = <DnsName as AsRef<str>>::as_ref(&name).to_string();
    name.make_ascii_lowercase();

    self.certs.insert(name, cert)
  }
}

impl ResolvesServerCert for CertStore {
  fn resolve(&self, client_hello: ClientHello) -> Option<Arc<CertifiedKey>> {
    let name = match client_hello.server_name() {
      Some(name) => name,
      None => &self.fallback_name,
    };

    match self.certs.get(name) {
      Some(key) => Some(key.clone()),
      None => None,
    }
  }
}

// Load public certificate from file.
pub fn load_certs(filename: &str) -> io::Result<Vec<rustls::Certificate>> {
  // Open certificate file.
  let certfile = File::open(filename)?;
  let mut reader = BufReader::new(certfile);

  // Load and return certificate.
  let certs = rustls_pemfile::certs(&mut reader)?;
  Ok(certs.into_iter().map(rustls::Certificate).collect())
}

// Load private key from file.
pub fn load_private_key(filename: &str) -> io::Result<Arc<dyn SigningKey>> {
  // Open keyfile.
  let keyfile = File::open(filename)?;
  let mut reader = BufReader::new(keyfile);

  // Load and return a single private key.
  loop {
    let key = match rustls_pemfile::read_one(&mut reader)? {
      None => break,
      Some(Item::RSAKey(data)) => PrivateKey(data),
      Some(Item::ECKey(data)) => PrivateKey(data),
      Some(Item::PKCS8Key(data)) => PrivateKey(data),
      _ => continue,
    };

    return Ok(
      any_supported_type(&key).expect("todo: something went wrong while loading private key"),
    );
  }

  panic!("missing private key");
}
