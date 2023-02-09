use lazy_static::lazy_static;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

const TESTNET: &str = "net.evercloud.dev";
const MAINNET: &str = "main.evercloud.dev";
pub const LOCALNET: &str = "http://127.0.0.1/";

fn default_url() -> String {
    LOCALNET.to_string()
}

fn default_wc() -> i32 {
    0
}

fn default_retries() -> u8 {
    5
}

fn default_depool_fee() -> f32 {
    0.5
}

fn default_timeout() -> u32 {
    40000
}

fn default_out_of_sync() -> u32 {
    15
}

fn default_false() -> bool {
    false
}

fn default_true() -> bool {
    true
}

fn default_lifetime() -> u32 {
    60
}

fn default_endpoints() -> Vec<String> {
    vec![]
}

fn default_endpoints_map() -> BTreeMap<String, Vec<String>> {
    Config::default_map()
}

fn default_trace() -> String {
    "None".to_string()
}

fn default_config() -> Config {
    Config::new()
}

#[derive(Serialize, Deserialize, Clone, PartialEq)]
pub struct Config {
    #[serde(default = "default_url")]
    pub url: String,
    #[serde(default = "default_wc")]
    pub wc: i32,
    pub wallet: Option<String>,
    pub pubkey: Option<String>,
    pub keys_path: Option<String>,
    #[serde(default = "default_retries")]
    pub retries: u8,
    #[serde(default = "default_timeout")]
    pub timeout: u32,
    #[serde(default = "default_timeout")]
    pub message_processing_timeout: u32,
    #[serde(default = "default_out_of_sync")]
    pub out_of_sync_threshold: u32,
    #[serde(default = "default_false")]
    pub is_json: bool,
    #[serde(default = "default_lifetime")]
    pub lifetime: u32,
    // SDK authentication parameters
    pub project_id: Option<String>,
    pub access_key: Option<String>,
    ////////////////////////////////
    #[serde(default = "default_endpoints")]
    pub endpoints: Vec<String>,
    #[serde(default = "default_endpoints_map")]
    pub endpoints_map: BTreeMap<String, Vec<String>>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            url: default_url(),
            wc: default_wc(),
            retries: default_retries(),
            timeout: default_timeout(),
            message_processing_timeout: default_timeout(),
            lifetime: default_lifetime(),
            endpoints: default_endpoints(),
            endpoints_map: default_endpoints_map(),
            out_of_sync_threshold: default_out_of_sync(),
            ..Default::default()
        }
    }
}

impl Config {
    fn new() -> Self {
        let url = default_url();
        let endpoints = Config::default_map()[&url].clone();
        Config {
            url,
            wc: default_wc(),
            retries: default_retries(),
            timeout: default_timeout(),
            message_processing_timeout: default_timeout(),
            is_json: default_false(),
            lifetime: default_lifetime(),
            endpoints,
            out_of_sync_threshold: default_out_of_sync(),
            endpoints_map: Self::default_map(),
            ..Default::default()
        }
    }
    pub fn default_map() -> BTreeMap<String, Vec<String>> {
        [
            (MAINNET.to_owned(), MAIN_ENDPOINTS.to_owned()),
            (TESTNET.to_owned(), NET_ENDPOINTS.to_owned()),
            (LOCALNET.to_owned(), SE_ENDPOINTS.to_owned()),
        ]
        .iter()
        .cloned()
        .collect()
    }

    pub fn from_env() -> Self {
        Self::new()
    }
}

lazy_static! {
    static ref MAIN_ENDPOINTS: Vec<String> = vec!["https://mainnet.evercloud.dev".to_string()];
    static ref NET_ENDPOINTS: Vec<String> = vec!["https://devnet.evercloud.dev".to_string()];
    static ref SE_ENDPOINTS: Vec<String> = vec![
        "http://0.0.0.0".to_string(),
        "http://127.0.0.1".to_string(),
        "http://localhost".to_string(),
    ];
}

pub fn resolve_net_name(url: &str) -> Option<String> {
    let url_regex = Regex::new(r"^\s*(?:https?://)?(?P<net>\w+\.evercloud\.dev)\s*")
        .expect("Regex compilation error");
    let ton_url_regex = Regex::new(r"^\s*(?:https?://)?(?P<net>\w+\.ton\.dev)\s*")
        .expect("Regex compilation error");
    let everos_url_regex = Regex::new(r"^\s*(?:https?://)?(?P<net>\w+\.everos\.dev)\s*")
        .expect("Regex compilation error");
    let mut net = None;
    for regex in [url_regex, ton_url_regex, everos_url_regex] {
        if let Some(captures) = regex.captures(url) {
            net = Some(
                captures
                    .name("net")
                    .expect("Unexpected: capture <net> was not found")
                    .as_str()
                    .replace("ton", "evercloud")
                    .replace("everos", "evercloud"),
            );
        }
    }
    if let Some(net) = net {
        if Config::default_map().contains_key(&net) {
            return Some(net);
        }
    }
    if url == "main" {
        return Some(MAINNET.to_string());
    }
    if url == "dev" || url == "devnet" {
        return Some(TESTNET.to_string());
    }
    if url.contains("127.0.0.1") || url.contains("0.0.0.0") || url.contains("localhost") {
        return Some(LOCALNET.to_string());
    }
    None
}
