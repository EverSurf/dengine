mod action;
mod activity;
mod bridge_api;
mod browser;
mod builtin_interfaces;
pub mod calltype;
mod common;
mod context;
mod debot_abi;
mod dengine;
mod errors;
mod helpers;
mod info;
mod json_interface;
pub mod prelude;
mod routines;
mod run_output;
mod sdk_prelude;

use crate::bridge_api::{fetch, remove, send, start};
use crate::common::{DInfo, Deserialize, Error, Serialize};

/// [UNSTABLE](UNSTABLE.md) Describes `Debot` metadata.
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq, api_derive::ApiType)]
pub struct DebotInfo {
    /// DeBot short name.
    pub name: Option<String>,
    /// DeBot semantic version.
    pub version: Option<String>,
    /// The name of DeBot deployer.
    pub publisher: Option<String>,
    /// Short info about DeBot.
    pub caption: Option<String>,
    /// The name of DeBot developer.
    pub author: Option<String>,
    /// TON address of author for questions and donations.
    pub support: Option<String>,
    /// String with the first messsage from DeBot.
    pub hello: Option<String>,
    /// String with DeBot interface language (ISO-639).
    pub language: Option<String>,
    /// String with DeBot ABI.
    pub dabi: Option<String>,
    /// DeBot icon.
    pub icon: Option<String>,
    /// Vector with IDs of DInterfaces used by DeBot.
    pub interfaces: Vec<String>,
    /// ABI version ("x.y") supported by DeBot
    #[serde(rename = "dabiVersion")]
    pub dabi_version: String,
}

impl From<DInfo> for DebotInfo {
    fn from(info: DInfo) -> Self {
        Self {
            name: info.name,
            version: info.version,
            publisher: info.publisher,
            caption: info.caption,
            author: info.author,
            support: info.support,
            hello: info.hello,
            language: info.language,
            dabi: info.dabi,
            icon: info.icon,
            interfaces: info.interfaces,
            dabi_version: info.dabi_version,
        }
    }
}

pub const DEBOT_WC: i8 = -31; // 0xDB
type JsonValue = serde_json::Value;
type TonClient = std::sync::Arc<ton_client::ClientContext>;
