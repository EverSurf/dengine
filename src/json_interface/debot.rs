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
use ton_client::error::ClientResult;
use crate::prelude::{DAction, Error, BrowserCallbacks, DebotActivity};
use crate::bridge_api::{ParamsOfInit, RegisteredDebot};
use ton_client::crypto::SigningBoxHandle;
use serde_derive::{Serialize, Deserialize};
use log::error;
use api_derive::{ApiType, api_function};

/// Returning values from Debot Browser callbacks.
#[derive(Serialize, Deserialize, Clone, Debug, ApiType)]
#[serde(tag="type")]
pub enum ResultOfAppDebotBrowser {
    /// Result of user input.
    Input {
        /// String entered by user.
        value: String
    },
    /// Result of getting signing box.
    GetSigningBox {
        /// Signing box for signing data requested by debot engine. Signing box is owned and disposed by debot engine
        signing_box: SigningBoxHandle
    },
    /// Result of `approve` callback.
    Approve {
        /// Indicates whether the DeBot is allowed to perform the specified operation.
        approved: bool,
    }
}

/// [UNSTABLE](UNSTABLE.md) [DEPRECATED](DEPRECATED.md) Debot Browser callbacks
///
/// Called by debot engine to communicate with debot browser.
#[derive(Serialize, Deserialize, Clone, Debug, ApiType)]
#[serde(tag="type")]
pub enum ParamsOfAppDebotBrowser {
    /// Print message to user.
    Log {
        /// A string that must be printed to user.
        msg: String
    },
    Input {
        /// A prompt string that must be printed to user before input request.
        prompt: String
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

 #[async_trait::async_trait]
 impl BrowserCallbacks for DebotBrowserAdapter {

     async fn log(&self, msg: String) {
         self.app_object.notify(ParamsOfAppDebotBrowser::Log { msg });
     }

     async fn switch(&self, ctx_id: u8) {
    }

   async fn switch_completed(&self) {
   }

    async fn show_action(&self, _act: DAction) {
    }

    async fn invoke_debot(&self, _debot: String, _action: DAction) -> Result<(), String> {
        Ok(())
    }

     async fn input(&self, prompt: &str, value: &mut String) {
         let response = self.app_object.call(ParamsOfAppDebotBrowser::Input {
                 prompt: prompt.to_owned(),
             })
             .await;
         match response {
             Ok(r) => match r {
                 ResultOfAppDebotBrowser::Input { value: v } => *value = v,
                 _ => error!("unexpected debot browser response: {:?}", r),
             },
             Err(e) => error!("debot browser failed to show action: {}", e),
         }
     }

     async fn get_signing_box(&self) -> Result<SigningBoxHandle, String> {
         let response = self.app_object.call(ParamsOfAppDebotBrowser::GetSigningBox)
             .await
             .map_err(|err| format!("debot browser failed to load keys: {}", err))?;

        match response {
            ResultOfAppDebotBrowser::GetSigningBox { signing_box } => Ok(signing_box),
            _ => Err(ton_client::client::Error::unexpected_callback_response(
                "GetSigningBox", response).to_string()),
        }
     }

    async fn send(&self, message: String) {
        self.app_object.notify(ParamsOfAppDebotBrowser::Send { message });
    }

    async fn approve(&self, activity: DebotActivity) -> ClientResult<bool> {
        let response = self.app_object.call(ParamsOfAppDebotBrowser::Approve { activity }).await?;

        match response {
            ResultOfAppDebotBrowser::Approve{approved} => Ok(approved),
            _ => Err(Error::browser_callback_failed("unexpected response")),
        }
    }
}

/// [UNSTABLE](UNSTABLE.md) [DEPRECATED](DEPRECATED.md) Creates and instance of DeBot.
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