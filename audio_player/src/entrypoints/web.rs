//! Macro that generates an entry point for a player using the internet.

/// Plays the songs from a specified URL.
#[macro_export]
macro_rules! web {
    (impl true) => {
        {
            use rustls_pki_types::pem::PemObject;
            use ureq::Agent;

            let cert = rustls_pki_types::CertificateDer::from_pem_slice(include_bytes!("cert.pem"))?;
            let mut root_store = rustls::RootCertStore::empty();
            root_store.add(cert)?;

            let tls_config = rustls::ClientConfig::builder_with_provider(
                rustls::crypto::ring::default_provider().into(),
            )
            .with_protocol_versions(&[&rustls::version::TLS12, &rustls::version::TLS13])?
            .with_root_certificates(root_store)
            .with_no_client_auth();
            ureq::builder().tls_config(Arc::new(tls_config)).build()
        }
    };
    (impl false) => {
        {
            use ureq::Agent;

            Agent::new()
        }
    };
    (impl) => {
        {
            use ureq::Agent;

            Agent::new()
        }
    };
    ($url:expr, $($freebox:tt)*) => {
        use std::sync::Arc;
        use ureq::Agent;
        use url::Url;
        use $crate::player::play_songs;
        use $crate::song::{EBox, Web};
        use $crate::web_utils::get_files;

        const URL: &str = $url;

        fn main() -> Result<(), EBox> {
            let agent: Agent = web!(impl $($freebox)*);
            let url = Url::parse(URL)?;
            let files = get_files(&agent, &url)?;

            let mut songs = files
                .iter()
                .map(|url| Web::new(url, &agent))
                .collect::<Vec<_>>();
            play_songs(&mut songs[..])
        }
    };
}
