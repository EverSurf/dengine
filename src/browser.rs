use crate::action::DAction;
use crate::activity::DebotActivity;
use crate::sdk_prelude::*;
use api_derive::ApiType;
use serde_derive::{Deserialize, Serialize};

pub type BrowserRef = Arc<dyn BrowserCallbacks + Send + Sync>;

#[derive(Serialize, Deserialize, Debug, Clone, ApiType, Default, PartialEq)]
#[repr(usize)]
pub enum LogLevel {
    #[default]
    User,
    Error,
    Warn,
    Debug,
    Trace,
}

//#[macro_export(local_inner_macros)]
macro_rules! log {
    ($browser:expr, $level:expr, $($arg:tt)+) => ($browser.log($level, format!($($arg)+)));
}

macro_rules! debug {
    ($browser:expr, $($arg:tt)+) => (log!($browser, LogLevel::Debug, $($arg)+));
}

macro_rules! error {
    ($browser:expr, $($arg:tt)+) => (log!($browser, LogLevel::Error, $($arg)+));
}

pub(crate) use {debug, log, error};

#[derive(Serialize, Deserialize, Debug, Clone, ApiType, Default)]
pub struct FetchHeader {
    pub key: String,
    pub value: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, ApiType, Default)]
pub struct FetchResponse {
    pub status: u16,
    pub headers: Vec<FetchHeader>,
    pub content: String,
}

#[derive(Serialize, Deserialize, ApiType, Default, Debug, Clone)]
pub struct WaitForTransactionParams {
    pub abi: Option<Abi>,
    pub message: String,
    pub shard_block_id: String,
    #[serde(default)]
    pub send_events: bool,
    pub sending_endpoints: Option<Vec<String>>,
}

/// Callbacks that are called by debot engine to communicate with Debot Browser.
#[async_trait::async_trait]
pub trait BrowserCallbacks {
    /// Prints text message to user.
    fn log(&self, level: LogLevel, msg: String);
    /// Requests keys from user.
    async fn get_signing_box(&self) -> Result<SigningBoxHandle, String>;
    /// Sends message with debot interface call to Browser.
    /// Message parameter is a BoC encoded as Base64.
    async fn send(&self, message: String);
    /// Requests permission to execute DeBot operation
    /// (e.g. sending messages to blockchain).
    async fn approve(&self, activity: DebotActivity) -> ClientResult<bool>;
    /// Network http(s) request
    async fn fetch(
        &self,
        url: String,
        method: String,
        headers: Vec<FetchHeader>,
        body: Option<String>,
    ) -> ClientResult<FetchResponse>;
    /// Data encryption.
    /// data - encoded as base64.
    /// Result - encrypted string as base64.
    async fn encrypt(&self, handle: EncryptionBoxHandle, data: String) -> ClientResult<String>;
    /// Data decryption
    async fn decrypt(&self, handle: EncryptionBoxHandle, data: String) -> ClientResult<String>;
    /// Data signing
    /// data - string with data to sign encoded as base64.
    async fn sign(&self, handle: SigningBoxHandle, data: String) -> ClientResult<String>;
    /// Send external incoming message to blockchain
    /// message - base64 string with serialized message.
    async fn send_message(&self, message: String) -> ClientResult<ResultOfSendMessage>;
    async fn query(&self, params: ParamsOfQuery) -> ClientResult<ResultOfQuery>;
    async fn query_collection(
        &self,
        params: ParamsOfQueryCollection,
    ) -> ClientResult<ResultOfQueryCollection>;
    async fn wait_for_collection(
        &self,
        params: ParamsOfWaitForCollection,
    ) -> ClientResult<ResultOfWaitForCollection>;
    async fn wait_for_transaction(
        &self,
        params: WaitForTransactionParams,
    ) -> ClientResult<ResultOfProcessMessage>;
    async fn query_transaction_tree(
        &self,
        params: ParamsOfQueryTransactionTree,
    ) -> ClientResult<ResultOfQueryTransactionTree>;
    async fn get_signing_box_info(&self, handle: SigningBoxHandle) -> ClientResult<String>;
    async fn get_encryption_box_info(
        &self,
        handle: EncryptionBoxHandle,
    ) -> ClientResult<EncryptionBoxInfo>;

    /// [Deprecated]
    async fn switch(&self, ctx_id: u8);
    /// [Deprecated]
    async fn switch_completed(&self);
    /// [Deprecated]
    async fn show_action(&self, act: DAction);
    /// [Deprecated]
    async fn input(&self, prompt: &str, value: &mut String);
    /// [Deprecated]
    async fn invoke_debot(&self, debot: String, action: DAction) -> Result<(), String>;
}
