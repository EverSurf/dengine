use crate::action::DAction;
use crate::activity::DebotActivity;
use crate::sdk_prelude::*;
use api_derive::ApiType;
use serde_derive::{Serialize, Deserialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Clone, ApiType)]
pub struct FetchResponse {
    status: u16,
    headers: Vec<String>,
    content: String,
}

/// Callbacks that are called by debot engine to communicate with Debot Browser.
#[async_trait::async_trait]
pub trait BrowserCallbacks {
    /// Prints text message to user.
    async fn log(&self, msg: String);
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
        headers: HashMap<String, String>,
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
    async fn query_collection(&self, params: ParamsOfQueryCollection) -> ClientResult<ResultOfQueryCollection>;

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
