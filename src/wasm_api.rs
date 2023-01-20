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
pub use calltype::prepare_ext_in_message;

pub use action::DAction;
pub use activity::{DebotActivity, Spending};
pub use browser::BrowserCallbacks;
pub use context::{DContext, STATE_EXIT, STATE_ZERO};
pub use dengine::DEngine;
pub use dinterface::{DebotInterface, DebotInterfaceExecutor, InterfaceResult};
pub use errors::{Error, ErrorCode};
use info::DInfo;
use crate::error::ClientResult;
use crate::ClientContext;
use std::sync::Arc;
use tokio::sync::Mutex;

/// [UNSTABLE](UNSTABLE.md) Parameters to start DeBot.
/// DeBot must be already initialized with init() function.
#[derive(Serialize, Deserialize, Default, ApiType)]
pub struct ParamsOfStart {
    /// Debot handle which references an instance of debot engine.
    debot_handle: DebotHandle,
}

/// [UNSTABLE](UNSTABLE.md) Starts the DeBot.
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
pub async fn start(
    context: Arc<ClientContext>,
    params: ParamsOfStart,
) -> ClientResult<()> {
    let mutex = context
        .debots
        .get(&params.debot_handle.0)
        .ok_or(Error::invalid_handle(params.debot_handle.0))?;
    let mut dengine = mutex.1.lock().await;
    dengine.start().await.map_err(Error::start_failed)
}

/// [UNSTABLE](UNSTABLE.md) Parameters to fetch DeBot metadata.
#[derive(Serialize, Deserialize, Default, ApiType)]
pub struct ParamsOfFetch {
    /// Debot smart contract address.
    pub address: String,
}

/// [UNSTABLE](UNSTABLE.md)
#[derive(Serialize, Deserialize, Default, ApiType)]
pub struct ResultOfFetch {
    /// Debot metadata.
    pub info: DebotInfo,
}

/// [UNSTABLE](UNSTABLE.md) Fetches DeBot metadata from blockchain.
///
/// Downloads DeBot from blockchain and creates and fetches its metadata.
#[api_function]
pub async fn fetch(
    context: Arc<ClientContext>,
    params: ParamsOfFetch,
) -> ClientResult<ResultOfFetch> {
    Ok(ResultOfFetch {
        info : DEngine::fetch(context, params.address).await.map_err(Error::fetch_failed)?.into()
    })
}

/// [UNSTABLE](UNSTABLE.md) Parameters to init DeBot.
#[derive(Serialize, Deserialize, Default, ApiType)]
pub struct ParamsOfInit {
    /// Debot smart contract address
    pub address: String,
}

/// [UNSTABLE](UNSTABLE.md) Structure for storing debot handle returned from `init` function.
#[derive(Serialize, Deserialize, ApiType, Default)]
pub struct RegisteredDebot {
    /// Debot handle which references an instance of debot engine.
    pub debot_handle: DebotHandle,
    /// Debot abi as json string.
    pub debot_abi: String,
    /// Debot metadata.
    pub info: DebotInfo,
}

/// [UNSTABLE](UNSTABLE.md) Creates an instance of DeBot.
///
/// Downloads DeBot smart contract (code and data) from blockchain and creates
/// an instance of Debot Engine for it.
/// Returns a debot handle which can be used later in `start`, `execute` or `send` functions.
/// # Remarks
/// It does not switch debot to context 0. Browser Callbacks are not called.
/// Can be used to invoke DeBot without starting.
pub async fn init(
    context: Arc<ClientContext>,
    params: ParamsOfInit,
    callbacks: impl BrowserCallbacks + Send + Sync + 'static,
) -> ClientResult<RegisteredDebot> {
    let mut dengine =
        DEngine::new_with_client(params.address, None, context.clone(), Arc::new(callbacks));
    let info: DebotInfo = dengine.init().await.map_err(Error::fetch_failed)?.into();

    let handle = context.get_next_id();
    context.debots.insert(handle, Mutex::new(dengine));
    let debot_abi = info.dabi.clone().unwrap_or(String::new());
    Ok(RegisteredDebot { debot_handle: DebotHandle(handle), info, debot_abi })
}

/// [UNSTABLE](UNSTABLE.md)
#[derive(Serialize, Deserialize, ApiType, Default)]
pub struct ParamsOfRemove {
    /// Debot handle which references an instance of debot engine.
    pub debot_handle: DebotHandle,
}

/// [UNSTABLE](UNSTABLE.md) Destroys debot handle.
///
/// Removes handle from Client Context and drops debot engine referenced by that handle.
#[wasm_bindgen]
pub fn remove(context: Arc<ClientContext>, params: ParamsOfRemove) -> ClientResult<()> {
    context.debots.remove(&params.debot_handle.0);
    Ok(())
}

/// [UNSTABLE](UNSTABLE.md) Parameters of `send` function.
#[derive(Serialize, Deserialize, ApiType, Default)]
pub struct ParamsOfSend {
    /// Debot handle which references an instance of debot engine.
    pub debot_handle: DebotHandle,
    /// BOC of internal message to debot encoded in base64 format.
    pub message: String,
}

/// [UNSTABLE](UNSTABLE.md) Sends message to Debot.
///
/// Used by Debot Browser to send response on Dinterface call or from other Debots.
#[wasm_bindgen]
pub async fn send(context: Arc<ClientContext>, params: ParamsOfSend) -> ClientResult<()> {
    let mutex = context
        .debots
        .get(&params.debot_handle.0)
        .ok_or(Error::invalid_handle(params.debot_handle.0))?;
    let mut dengine = mutex.1.lock().await;
    dengine
        .send(params.message)
        .await
}
