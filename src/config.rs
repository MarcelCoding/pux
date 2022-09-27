use std::net::SocketAddr;

use serde::Deserialize;

#[derive(Deserialize)]
pub struct Config {
  #[serde(default)]
  pub entrypoints: Vec<EntrypointConfig>,
  #[serde(default)]
  pub routes: Vec<RouteConfig>,
  pub services: ServiceConfig,
  #[serde(default)]
  pub upstreams: Vec<UpstreamConfig>,
  #[serde(default)]
  pub certs: Vec<CertificateConfig>,
}

#[derive(Deserialize)]
pub struct EntrypointConfig {
  pub id: String,
  pub addr: SocketAddr,
  pub tls: bool,
}

#[derive(Deserialize)]
pub struct RouteConfig {
  pub host: String,
  #[serde(default)]
  pub path: Vec<String>,
  pub entrypoints: Vec<String>,
  pub service: String,
}

#[derive(Deserialize)]
pub struct ServiceConfig {
  pub proxy: Vec<ProxyServiceConfig>,
}

#[derive(Deserialize)]
pub struct ProxyServiceConfig {
  pub id: String,
  pub upstream: String,
}

#[derive(Deserialize)]
pub struct UpstreamConfig {
  pub id: String,
  pub addrs: Vec<SocketAddr>,
  pub sni: Option<String>,
}

#[derive(Deserialize)]
pub struct CertificateConfig {
  pub names: Vec<String>,
  pub chain: String,
  pub key: String,
}
