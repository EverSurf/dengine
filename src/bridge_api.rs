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
*/

#[cfg(not(feature = "wasm-base"))]
pub use crate::calltype::prepare_ext_in_message;

use crate::json_interface::DengineContext;
use crate::prelude::*;
use api_derive::{api_function, ApiType};
use serde_derive::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;
use ton_client::client::{ClientConfig, ClientContext};
use ton_client::error::ClientResult;
use ton_client::net::NetworkConfig;

#[derive(Serialize, Deserialize, Default, ApiType, Clone)]
pub struct DebotHandle(u32);

///  Parameters to start DeBot.
/// DeBot must be already initialized with init() function.
#[derive(Serialize, Deserialize, Default, ApiType)]
pub struct ParamsOfStart {
    /// Debot handle which references an instance of debot engine.
    debot_handle: DebotHandle,
}

///  Starts the DeBot.
///
/// Downloads debot smart contract from blockchain and switches it to
/// context zero.
///
/// This function must be used by Debot Browser to start a dialog with debot.
/// While the function is executing, several Browser Callbacks can be called,
/// since the debot tries to display all actions from the context 0 to the user.
///
/// When the debot starts SDK registers `BrowserCallbacks` AppObject.
/// Therefore when `debote.remove` is called the debot is being deleted and the callback is called
/// with `finish`=`true` which indicates that it will never be used again.
#[api_function]
pub async fn start(context: Arc<DengineContext>, params: ParamsOfStart) -> ClientResult<()> {
    let mutex = context
        .debots
        .get(&params.debot_handle.0)
        .ok_or(Error::invalid_handle(params.debot_handle.0))?;
    let mut dengine = mutex.1.lock().await;
    dengine.start().await.map_err(Error::start_failed)
}

///  Parameters to fetch DeBot metadata.
#[derive(Serialize, Deserialize, Default, ApiType)]
pub struct ParamsOfFetch {
    /// Debot smart contract address.
    pub address: String,
}

///
#[derive(Serialize, Deserialize, Default, ApiType)]
pub struct ResultOfFetch {
    /// Debot metadata.
    pub info: DebotInfo,
}

///  Fetches DeBot metadata from blockchain.
///
/// Downloads DeBot from blockchain and creates and fetches its metadata.
#[api_function]
pub async fn fetch(
    context: Arc<DengineContext>,
    params: ParamsOfFetch,
) -> ClientResult<ResultOfFetch> {
    let conf = ClientConfig {
        network: NetworkConfig {
            endpoints: context.endpoints.clone(),
            access_key: context.access_key.clone(),
            ..Default::default()
        },
        ..Default::default()
    };
    let cli = ClientContext::new(conf)?;
    Ok(ResultOfFetch {
        info: DEngine::fetch(Arc::new(cli), params.address)
            .await
            .map_err(Error::fetch_failed)?
            .into(),
    })
}

///  Parameters to init DeBot.
#[derive(Serialize, Deserialize, Default, ApiType)]
pub struct ParamsOfInit {
    /// Debot smart contract address
    pub address: String,
}

///  Structure for storing debot handle returned from `init` function.
#[derive(Serialize, Deserialize, ApiType, Default)]
pub struct RegisteredDebot {
    /// Debot handle which references an instance of debot engine.
    pub debot_handle: DebotHandle,
    /// Debot abi as json string.
    pub debot_abi: String,
    /// Debot metadata.
    pub info: DebotInfo,
}

///  Creates an instance of DeBot.
///
/// Downloads DeBot smart contract (code and data) from blockchain and creates
/// an instance of Debot Engine for it.
/// Returns a debot handle which can be used later in `start`, or `send` functions.
/// # Remarks
/// Can be used to invoke DeBot without starting.
pub async fn init(
    context: Arc<DengineContext>,
    params: ParamsOfInit,
    callbacks: impl BrowserCallbacks + Send + Sync + 'static,
) -> ClientResult<RegisteredDebot> {
    let mut dengine = DEngine::new(
        params.address,
        None,
        context.endpoints.clone(),
        Arc::new(callbacks),
    );
    let info: DebotInfo = dengine.init().await.map_err(Error::fetch_failed)?.into();

    let handle = context.get_next_id();
    context.debots.insert(handle, Mutex::new(dengine));
    let debot_abi = info.dabi.clone().unwrap_or(String::new());
    Ok(RegisteredDebot {
        debot_handle: DebotHandle(handle),
        info,
        debot_abi,
    })
}

///
#[derive(Serialize, Deserialize, ApiType, Default)]
pub struct ParamsOfRemove {
    /// Debot handle which references an instance of debot engine.
    pub debot_handle: DebotHandle,
}

///  Destroys debot handle.
///
/// Removes handle from Client Context and drops debot engine referenced by that handle.
//#[wasm_bindgen]
#[api_function]
pub fn remove(context: Arc<DengineContext>, params: ParamsOfRemove) -> ClientResult<()> {
    context.debots.remove(&params.debot_handle.0);
    Ok(())
}

///  Parameters of `send` function.
#[derive(Serialize, Deserialize, ApiType, Default)]
pub struct ParamsOfSend {
    /// Debot handle which references an instance of debot engine.
    pub debot_handle: DebotHandle,
    /// BOC of internal message to debot encoded in base64 format.
    pub message: String,
}

///  Sends message to Debot.
///
/// Used by Debot Browser to send response on Dinterface call or from other Debots.
//#[wasm_bindgen]
#[api_function]
pub async fn send(context: Arc<DengineContext>, params: ParamsOfSend) -> ClientResult<()> {
    let mutex = context
        .debots
        .get(&params.debot_handle.0)
        .ok_or(Error::invalid_handle(params.debot_handle.0))?;
    let mut dengine = mutex.1.lock().await;
    dengine.send(params.message).await
}
