/*
 * Copyright 2018-2021 TON Labs LTD.
 *
 * Licensed under the SOFTWARE EVALUATION License (the "License"); you may not use
 * this file except in compliance with the License.
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific TON DEV software governing permissions and
 * limitations under the License.
 *
 */

use super::client::{AppObject, DengineContext};
use crate::bridge_api::{ParamsOfInit, RegisteredDebot};
use crate::prelude::{BrowserCallbacks, FetchHeader, FetchResponse, DAction, DebotActivity, Error, WaitForTransactionParams};
use api_derive::{api_function, ApiType};
use serde_derive::{Deserialize, Serialize};
use crate::sdk_prelude::*;

/// Returning values from Debot Browser callbacks.
#[derive(Serialize, Deserialize, Clone, ApiType)]
#[serde(tag = "type")]
pub enum ResultOfAppDebotBrowser {
    /// Result of getting signing box.
    GetSigningBox {
        /// Signing box for signing data requested by debot engine. Signing box is owned and disposed by debot engine
        signing_box: SigningBoxHandle,
    },
    /// Result of `approve` callback.
    Approve {
        /// Indicates whether the DeBot is allowed to perform the specified operation.
        approved: bool,
    },
    Fetch {
        response: FetchResponse,
    },
    Encrypt {
        encrypted: String,
    },
    Decrypt {
        decrypted: String,
    },
    Sign {
        signature: String,
    },
    SendMessage {
        shard_block_id: String,
        sending_endpoints: Vec<String>,
    },
    Query {
        result: ResultOfQuery,
    },
    QueryCollection {
        result: ResultOfQueryCollection,
    },
    WaitForCollection {
        result: ResultOfWaitForCollection,
    },
    WaitForTransaction {
        result: ResultOfProcessMessage,
    },
    QueryTransactionTree {
        result: ResultOfQueryTransactionTree,
    },
    GetSigningBoxInfo {
        pubkey: String
    },
    GetEncryptionBoxInfo{
        result: EncryptionBoxInfo
    }
}

///  [DEPRECATED](DEPRECATED.md) Debot Browser callbacks
///
/// Called by debot engine to communicate with debot browser.
#[derive(Serialize, Deserialize, Clone, ApiType)]
#[serde(tag = "type")]
pub enum ParamsOfAppDebotBrowser {
    /// Print message to user.
    Log {
        /// A string that must be printed to user.
        msg: String,
    },
    /// Get signing box to sign data. Signing box returned is owned and disposed by debot engine
    GetSigningBox,
    /// Used by Debot to call DInterface implemented by Debot Browser.
    Send {
        /// Internal message to DInterface address. Message body contains
        /// interface function and parameters.
        message: String,
    },
    /// Requests permission from DeBot Browser to execute DeBot operation.
    Approve {
        /// DeBot activity details.
        activity: DebotActivity,
    },
    Fetch {
        url: String,
        method: String,
        headers: Vec<FetchHeader>,
        body: Option<String>,
    },
    Encrypt {
        handle: EncryptionBoxHandle,
        data: String,
    },
    Decrypt {
        handle: EncryptionBoxHandle,
        data: String,
    },
    Sign {
        handle: SigningBoxHandle,
        data: String,
    },
    SendMessage {
        message: String,
    },
    Query {
        params: ParamsOfQuery,
    },
    QueryCollection {
        params: ParamsOfQueryCollection,
    },
    WaitForCollection {
        params: ParamsOfWaitForCollection,
    },
    WaitForTransaction {
        params: WaitForTransactionParams,
    },
    QueryTransactionTree {
        params: ParamsOfQueryTransactionTree,
    },
    GetSigningBoxInfo {
        handle: SigningBoxHandle,
    },
    GetEncryptionBoxInfo {
        handle: EncryptionBoxHandle,
    }
}

/// Wrapper for native Debot Browser callbacks.
///
/// Adapter between SDK application and low level debot interface.
pub(crate) struct DebotBrowserAdapter {
    app_object: AppObject<ParamsOfAppDebotBrowser, ResultOfAppDebotBrowser>,
}

impl DebotBrowserAdapter {
    pub fn new(app_object: AppObject<ParamsOfAppDebotBrowser, ResultOfAppDebotBrowser>) -> Self {
        Self { app_object }
    }
}

fn unexpected_response_err() -> ClientError {
    Error::browser_callback_failed("unexpected response")
}

#[async_trait::async_trait]
impl BrowserCallbacks for DebotBrowserAdapter {
    async fn log(&self, msg: String) {
        self.app_object.notify(ParamsOfAppDebotBrowser::Log { msg });
    }

    async fn switch(&self, _ctx_id: u8) {}

    async fn switch_completed(&self) {}

    async fn show_action(&self, _act: DAction) {}

    async fn input(&self, _prompt: &str, _value: &mut String) {}

    async fn invoke_debot(&self, _debot: String, _action: DAction) -> Result<(), String> {
        Ok(())
    }

    async fn get_signing_box(&self) -> Result<SigningBoxHandle, String> {
        let response = self
            .app_object
            .call(ParamsOfAppDebotBrowser::GetSigningBox)
            .await
            .map_err(|err| format!("debot browser failed to load keys: {}", err))?;

        match response {
            ResultOfAppDebotBrowser::GetSigningBox { signing_box } => Ok(signing_box),
            _ => Err(unexpected_response_err().to_string()),
        }
    }

    async fn send(&self, message: String) {
        self.app_object
            .notify(ParamsOfAppDebotBrowser::Send { message });
    }

    async fn approve(&self, activity: DebotActivity) -> ClientResult<bool> {
        let response = self
            .app_object
            .call(ParamsOfAppDebotBrowser::Approve { activity })
            .await?;

        match response {
            ResultOfAppDebotBrowser::Approve { approved } => Ok(approved),
            _ => Err(unexpected_response_err()),
        }
    }

    async fn fetch(
        &self,
        url: String,
        method: String,
        headers: Vec<FetchHeader>,
        body: Option<String>,
    ) -> ClientResult<FetchResponse> {
        let response = self
            .app_object
            .call(ParamsOfAppDebotBrowser::Fetch { url, method, headers, body })
            .await?;

        match response {
            ResultOfAppDebotBrowser::Fetch { response } => Ok(response),
            _ => Err(unexpected_response_err()),
        }
    }

    async fn encrypt(
        &self,
        handle: EncryptionBoxHandle,
        data: String,
    ) -> ClientResult<String> {
        let response = self
            .app_object
            .call(ParamsOfAppDebotBrowser::Encrypt { handle, data })
            .await?;
        match response {
            ResultOfAppDebotBrowser::Encrypt { encrypted } => Ok(encrypted),
            _ => Err(unexpected_response_err()),
        }
    }

    async fn decrypt(
        &self,
        handle: EncryptionBoxHandle,
        data: String,
    ) -> ClientResult<String> {
        let response = self
            .app_object
            .call(ParamsOfAppDebotBrowser::Decrypt { handle, data })
            .await?;
        match response {
            ResultOfAppDebotBrowser::Decrypt { decrypted } => Ok(decrypted),
            _ => Err(unexpected_response_err()),
        }
    }

    async fn sign(
        &self,
        handle: SigningBoxHandle,
        data: String,
    ) -> ClientResult<String> {
        let response = self
            .app_object
            .call(ParamsOfAppDebotBrowser::Sign { handle, data })
            .await?;
        match response {
            ResultOfAppDebotBrowser::Sign { signature } => Ok(signature),
            _ => Err(unexpected_response_err()),
        }
    }

    async fn send_message(
        &self,
        message: String,
    ) -> ClientResult<ResultOfSendMessage> {
        let response = self
            .app_object
            .call(ParamsOfAppDebotBrowser::SendMessage { message })
            .await?;
        match response {
            ResultOfAppDebotBrowser::SendMessage { 
                shard_block_id, sending_endpoints
            } => Ok(ResultOfSendMessage {shard_block_id, sending_endpoints}),
            _ => Err(unexpected_response_err()),
        }
    }

    async fn query(
        &self,
        params: ParamsOfQuery,
    ) -> ClientResult<ResultOfQuery> {
        let response = self
            .app_object
            .call(ParamsOfAppDebotBrowser::Query { params })
            .await?;
        match response {
            ResultOfAppDebotBrowser::Query { result } => Ok(result),
            _ => Err(unexpected_response_err()),
        }
    }

    async fn query_collection(
        &self,
        params: ParamsOfQueryCollection,
    ) -> ClientResult<ResultOfQueryCollection> {
        let response = self
            .app_object
            .call(ParamsOfAppDebotBrowser::QueryCollection { params })
            .await?;
        match response {
            ResultOfAppDebotBrowser::QueryCollection { result } => Ok(result),
            _ => Err(unexpected_response_err()),
        }
    }

    async fn wait_for_collection(
        &self,
        params: ParamsOfWaitForCollection,
    ) -> ClientResult<ResultOfWaitForCollection> {
        let response = self
            .app_object
            .call(ParamsOfAppDebotBrowser::WaitForCollection { params })
            .await?;
        match response {
            ResultOfAppDebotBrowser::WaitForCollection { result } => Ok(result),
            _ => Err(unexpected_response_err()),
        }
    }

    async fn wait_for_transaction(
        &self,
        params: WaitForTransactionParams,
    ) -> ClientResult<ResultOfProcessMessage> {
        let response = self
            .app_object
            .call(ParamsOfAppDebotBrowser::WaitForTransaction { params })
            .await?;
        match response {
            ResultOfAppDebotBrowser::WaitForTransaction { result } => Ok(result),
            _ => Err(unexpected_response_err()),
        }
    }

    async fn query_transaction_tree(
        &self,
        params: ParamsOfQueryTransactionTree,
    ) -> ClientResult<ResultOfQueryTransactionTree> {
        let response = self
            .app_object
            .call(ParamsOfAppDebotBrowser::QueryTransactionTree { params })
            .await?;
        match response {
            ResultOfAppDebotBrowser::QueryTransactionTree { result } => Ok(result),
            _ => Err(unexpected_response_err()),
        }
    }

    async fn get_signing_box_info(
        &self,
        handle: SigningBoxHandle,
    ) -> ClientResult<String> { 
        let response = self
            .app_object
            .call(ParamsOfAppDebotBrowser::GetSigningBoxInfo { handle })
            .await?;
        match response {
            ResultOfAppDebotBrowser::GetSigningBoxInfo { pubkey } => Ok(pubkey),
            _ => Err(unexpected_response_err()),
        }
    }

    async fn get_encryption_box_info(
        &self,
        handle: EncryptionBoxHandle,
    ) -> ClientResult<EncryptionBoxInfo> { 
        let response = self
            .app_object
            .call(ParamsOfAppDebotBrowser::GetEncryptionBoxInfo { handle })
            .await?;
        match response {
            ResultOfAppDebotBrowser::GetEncryptionBoxInfo { result } => Ok(result),
            _ => Err(unexpected_response_err()),
        }
    }
}

/// Creates and instance of DeBot.
///
/// Downloads debot smart contract (code and data) from blockchain and creates
/// an instance of Debot Engine for it.
///
/// # Remarks
/// It does not switch debot to context 0. Browser Callbacks are not called.
#[api_function]
pub(crate) async fn init(
    context: std::sync::Arc<DengineContext>,
    params: ParamsOfInit,
    app_object: AppObject<ParamsOfAppDebotBrowser, ResultOfAppDebotBrowser>,
) -> ClientResult<RegisteredDebot> {
    let browser_callbacks = DebotBrowserAdapter::new(app_object);
    crate::bridge_api::init(context, params, browser_callbacks).await
}
