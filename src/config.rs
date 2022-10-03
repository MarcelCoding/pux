use std::net::SocketAddr;

use serde::Deserialize;

#[derive(Deserialize)]
pub(crate) struct Config {
  #[serde(default)]
  pub(crate) entrypoints: Vec<EntrypointConfig>,
  #[serde(default)]
  pub(crate) routes: Vec<RouteConfig>,
  pub(crate) services: ServiceConfig,
  #[serde(default)]
  pub(crate) upstreams: Vec<UpstreamConfig>,
  #[serde(default)]
  pub(crate) certs: Vec<CertificateConfig>,
}

#[derive(Deserialize)]
pub(crate) struct EntrypointConfig {
  pub(crate) id: String,
  pub(crate) addr: SocketAddr,
  pub(crate) tls: bool,
}

#[derive(Deserialize)]
pub(crate) struct RouteConfig {
  pub(crate) host: String,
  #[serde(default)]
  pub(crate) path: Vec<String>,
  pub(crate) entrypoints: Vec<String>,
  pub(crate) service: String,
}

#[derive(Deserialize)]
pub(crate) struct ServiceConfig {
  pub(crate) proxy: Vec<ProxyServiceConfig>,
}

#[derive(Deserialize)]
pub(crate) struct ProxyServiceConfig {
  pub(crate) id: String,
  pub(crate) upstream: String,
}

#[derive(Deserialize)]
pub(crate) struct UpstreamConfig {
  pub(crate) id: String,
  pub(crate) addrs: Vec<SocketAddr>,
  pub(crate) sni: Option<String>,
}

#[derive(Deserialize)]
pub(crate) struct CertificateConfig {
  pub(crate) names: Vec<String>,
  pub(crate) chain: String,
  pub(crate) key: String,
}
