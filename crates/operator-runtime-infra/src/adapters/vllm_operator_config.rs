use std::path::PathBuf;
use std::time::Duration;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VllmOperatorConfig {
    base_url: String,
    model_id: String,
    client_cert_path: Option<PathBuf>,
    client_key_path: Option<PathBuf>,
    timeout: Duration,
    max_tokens: u32,
    accept_invalid_certs: bool,
}

impl VllmOperatorConfig {
    pub fn new(base_url: impl Into<String>, model_id: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            model_id: model_id.into(),
            client_cert_path: None,
            client_key_path: None,
            timeout: Duration::from_secs(60),
            max_tokens: 4096,
            accept_invalid_certs: false,
        }
    }

    #[must_use]
    pub fn with_client_identity(
        mut self,
        cert_path: impl Into<PathBuf>,
        key_path: impl Into<PathBuf>,
    ) -> Self {
        self.client_cert_path = Some(cert_path.into());
        self.client_key_path = Some(key_path.into());
        self
    }

    #[must_use]
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    #[must_use]
    pub fn with_max_tokens(mut self, max_tokens: u32) -> Self {
        self.max_tokens = max_tokens;
        self
    }

    #[must_use]
    pub fn with_accept_invalid_certs(mut self, accept_invalid_certs: bool) -> Self {
        self.accept_invalid_certs = accept_invalid_certs;
        self
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    pub fn model_id(&self) -> &str {
        &self.model_id
    }

    pub fn client_cert_path(&self) -> Option<&PathBuf> {
        self.client_cert_path.as_ref()
    }

    pub fn client_key_path(&self) -> Option<&PathBuf> {
        self.client_key_path.as_ref()
    }

    pub fn timeout(&self) -> Duration {
        self.timeout
    }

    pub fn max_tokens(&self) -> u32 {
        self.max_tokens
    }

    pub fn accept_invalid_certs(&self) -> bool {
        self.accept_invalid_certs
    }
}
