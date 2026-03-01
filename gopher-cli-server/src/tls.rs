use std::fs::File;
use std::io::{BufReader, Error, ErrorKind, Result};
use std::path::Path;
use std::sync::Arc;
use rustls::server::AllowAnyAuthenticatedClient;
use rustls::{RootCertStore, ServerConfig, Certificate, PrivateKey};
use rustls_pemfile::{certs, pkcs8_private_keys};

pub fn load_certs(path: &Path) -> Result<Vec<Certificate>> {
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);
    let certs_data = certs(&mut reader)?;
    let certs = certs_data
        .into_iter()
        .map(Certificate)
        .collect();
    Ok(certs)
}

pub fn load_private_key(path: &Path) -> Result<PrivateKey> {
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);
    let keys = pkcs8_private_keys(&mut reader)?;
    if keys.is_empty() {
        return Err(Error::new(ErrorKind::Other, "no private key found"));
    }
    Ok(PrivateKey(keys[0].clone()))
}

pub fn make_server_config(
    cert_path: &Path,
    key_path: &Path,
    client_ca_path: &Path,
) -> Result<Arc<ServerConfig>> {
    let certs = load_certs(cert_path)?;
    let key = load_private_key(key_path)?;
    let client_ca_certs = load_certs(client_ca_path)?;

    let mut root_store = RootCertStore::empty();
    for cert in client_ca_certs {
        root_store.add(&cert).map_err(|e| Error::new(ErrorKind::Other, format!("invalid CA cert: {}", e)))?;
    }

    let verifier = AllowAnyAuthenticatedClient::new(root_store);

    let config = ServerConfig::builder()
        .with_safe_defaults()
        .with_client_cert_verifier(Arc::new(verifier))
        .with_single_cert(certs, key)
        .map_err(|e| Error::new(ErrorKind::Other, e.to_string()))?;

    Ok(Arc::new(config))
}
