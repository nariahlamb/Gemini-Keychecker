use crate::error::ValidatorError;
use clap::Parser;
use figment::{
    Figment,
    providers::{Env, Format, Serialized, Toml},
};
use serde::{Deserialize, Serialize};
use std::fmt::{self, Display};
use std::fs;
use std::path::PathBuf;
use std::sync::LazyLock;
use url::Url;

/// Cli arguments
#[derive(Parser, Debug, Serialize, Deserialize)]
struct Cli {
    #[arg(short = 'i', long)]
    #[serde(skip_serializing_if = "Option::is_none")]
    input_path: Option<PathBuf>,

    #[arg(short = 'b', long)]
    #[serde(skip_serializing_if = "Option::is_none")]
    backup_path: Option<PathBuf>,

    #[arg(short = 'u', long)]
    #[serde(skip_serializing_if = "Option::is_none")]
    api_host: Option<Url>,

    #[arg(short = 't', long)]
    #[serde(skip_serializing_if = "Option::is_none")]
    timeout_sec: Option<u64>,

    #[arg(short = 'c', long)]
    #[serde(skip_serializing_if = "Option::is_none")]
    concurrency: Option<usize>,

    #[arg(short = 'r', long)]
    #[serde(skip_serializing_if = "Option::is_none")]
    max_retries: Option<usize>,

    #[arg(short = 'x', long)]
    #[serde(skip_serializing_if = "Option::is_none")]
    proxy: Option<Url>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyCheckerConfig {
    // Input file path containing API keys to check.
    #[serde(default)]
    pub input_path: PathBuf,

    // Backup file path for all API keys.
    #[serde(default)]
    pub backup_path: PathBuf,

    // API host URL for key validation.
    #[serde(default = "default_api_host")]
    pub api_host: Url,

    // Request timeout in seconds.
    #[serde(default)]
    pub timeout_sec: u64,

    // Maximum number of retries for failed requests.
    #[serde(default)]
    pub max_retries: usize,

    // Maximum number of concurrent requests.
    #[serde(default)]
    pub concurrency: usize,

    // Optional proxy URL for HTTP requests (e.g., --proxy http://user:pass@host:port).
    #[serde(default)]
    pub proxy: Option<Url>,

    // Whether to enable HTTP/2 multiplexing for requests.
    #[serde(default)]
    pub enable_multiplexing: bool,

    #[serde(default)]
    pub log_level: String,
}

impl Default for KeyCheckerConfig {
    fn default() -> Self {
        (*DEFAULT_CONFIG).clone()
    }
}
impl KeyCheckerConfig {
    pub fn load_config() -> Result<Self, ValidatorError> {
        // Define the path to the configuration file
        static CONFIG_PATH: LazyLock<PathBuf> = LazyLock::new(|| "Config.toml".into());

        // Check if config.toml exists, if not create it with default values
        if !CONFIG_PATH.exists() {
            let default_config = Self::default();
            let toml_content = toml::to_string_pretty(&default_config)?;
            fs::write(CONFIG_PATH.as_path(), toml_content)?;
        }

        // Load configuration from config.toml, environment variables, and CLI arguments
        let config: Self = Figment::new()
            .merge(Serialized::defaults(Self::default()))
            .merge(Toml::file(CONFIG_PATH.as_path()))
            .merge(Env::prefixed("KEYCHECKER_"))
            .merge(Serialized::defaults(Cli::parse()))
            .extract()?;

        Ok(config)
    }

    /// Returns the complete Gemini API URL for generateContent endpoint
    pub fn gemini_api_url(&self) -> Url {
        self.api_host
            .join("v1beta/models/gemini-3.1-flash-lite-preview:generateContent")
            .expect("Failed to join API URL")
    }

    /// Returns the complete Gemini API URL for cachedContents endpoint
    pub fn cache_api_url(&self) -> Url {
        self.api_host
            .join("v1beta/cachedContents")
            .expect("Failed to join cache API URL")
    }
}

impl Display for KeyCheckerConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let proxy_status = match &self.proxy {
            Some(proxy) => proxy.to_string(),
            None => "Disabled".to_string(),
        };

        let protocol_status = if self.enable_multiplexing {
            "HTTP/2 (Multiplexing Enabled)"
        } else {
            "HTTP/1.1 (Multiplexing Disabled)"
        };

        write!(
            f,
            "Host={}, Proxy={}, Protocol={}, Timeout={}s, Concurrency={}, Input={}, Backup={}",
            self.api_host,
            proxy_status,
            protocol_status,
            self.timeout_sec,
            self.concurrency,
            self.input_path.display(),
            self.backup_path.display()
        )
    }
}

// Single LazyLock for entire default configuration
static DEFAULT_CONFIG: LazyLock<KeyCheckerConfig> = LazyLock::new(|| KeyCheckerConfig {
    input_path: "keys.txt".into(),
    backup_path: "backup_keys.txt".into(),
    api_host: Url::parse("https://generativelanguage.googleapis.com/").unwrap(),
    timeout_sec: 15,
    concurrency: 50,
    max_retries: 2,
    proxy: None,
    enable_multiplexing: true,
    log_level: "info".to_string(),
});

fn default_api_host() -> Url {
    DEFAULT_CONFIG.api_host.clone()
}
