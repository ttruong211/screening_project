// Handles TLS connections using tokio-rustls 

use tokio_rustls::rustls::{ClientConfig, OwnedTrustAnchor, RootCertStore, ServerName};
use tokio_rustls::TlsConnector;
use tokio::net::TcpStream;
use tokio_rustls::client::TlsStream;
use std::sync::Arc;
use webpki_roots::TLS_SERVER_ROOTS;
use anyhow::{Result};

pub async fn tls_connect(stream: TcpStream, domain: ServerName) -> Result<TlsStream<TcpStream>> {
    let mut root_store = RootCertStore::empty();
    root_store.add_trust_anchors(TLS_SERVER_ROOTS.iter().map(|ta| {
        OwnedTrustAnchor::from_subject_spki_name_constraints(
            ta.subject,
            ta.spki, 
            ta.name_constraints,
        )
    })); 

    let config = ClientConfig::builder()
        .with_safe_defaults()
        .with_root_certificates(root_store)
        .with_no_client_auth();
    
    let connector = TlsConnector::from(Arc::new(config));
    let tls_stream = connector.connect(domain, stream).await?;

    Ok(tls_stream)
}