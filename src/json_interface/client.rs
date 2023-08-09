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

use lockfree::map::Map as LockfreeMap;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use tokio::sync::{oneshot, Mutex};
use crate::prelude::DEngine;
use ton_client::error::ClientResult;
use ton_client::client::Error;
use super::{interop::ResponseType, request::Request};
use api_derive::{ApiType, api_function};

use tokio::runtime::Runtime;
use api_info::API;
use futures::Future;
use lazy_static::lazy_static;

lazy_static! {
    static ref TOKIO_RUNTIME: ClientResult<Runtime> = 
        tokio::runtime::Builder::new_multi_thread()
            .enable_io()
            .enable_time()
            .build()
            .map_err(|err| Error::cannot_create_runtime(err));
}

#[derive(Clone, Default, Deserialize, Serialize, Debug, ApiType)]
pub struct BindingConfig {
    #[serde(default)]
    pub library: String,
    #[serde(default)]
    pub version: String,
}

pub struct DengineContext {
    next_id: AtomicU32,
    pub endpoints: Option<Vec<String>>,
    pub access_key: Option<String>,
    async_runtime_handle: tokio::runtime::Handle,
    // context
    #[allow(dead_code)]
    pub(crate) binding: BindingConfig,
    pub(crate) app_requests: Mutex<HashMap<u32, oneshot::Sender<AppRequestResult>>>,

    // debot module
    pub(crate) debots: LockfreeMap<u32, Mutex<DEngine>>,
}

impl std::fmt::Debug for DengineContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DengineContext").finish()
    }

}

impl DengineContext {
    pub fn new(endpoints: Option<Vec<String>>, access_key: Option<String>) -> ClientResult<DengineContext> {
        let async_runtime_handle = match tokio::runtime::Handle::try_current() {
            Ok(handle) => handle,
            Err(_) => TOKIO_RUNTIME
                .as_ref()
                .map_err(|err| err.clone())?
                .handle()
                .clone(),
        };

        Ok(Self {
            endpoints,
            access_key,
            async_runtime_handle,
            debots: LockfreeMap::new(),
            app_requests: Mutex::new(HashMap::new()),
            next_id: AtomicU32::new(1),
            binding: Default::default(),
        })
    }

    pub fn from_json_str(conf_str: &str) -> ClientResult<Self> {
        #[derive(Deserialize)]
        struct Conf {
            endpoints: Option<Vec<String>>,
            access_key: Option<String>,
        }
        let conf: Conf = serde_json::from_str(conf_str).
            map_err(|_| ton_client::client::Error::invalid_config(conf_str.to_owned()))?;
        DengineContext::new(conf.endpoints, conf.access_key)
    }

    pub fn spawn(&self, future: impl Future<Output = ()> + Send + 'static) {
        self.async_runtime_handle.spawn(future);
    }

    pub(crate) fn get_next_id(&self) -> u32 {
        self.next_id.fetch_add(1, Ordering::Relaxed)
    }

    pub(crate) async fn app_request<R: DeserializeOwned>(
        &self,
        callback: &Request,
        params: impl Serialize,
    ) -> ClientResult<R> {
        let id = self.get_next_id();
        let (sender, receiver) = oneshot::channel();
        self.app_requests.lock().await.insert(id, sender);

        let params = serde_json::to_value(params).map_err(Error::cannot_serialize_result)?;

        callback.response(
            ParamsOfAppRequest {
                app_request_id: id,
                request_data: params,
            },
            ResponseType::AppRequest as u32,
        );
        let result = receiver
            .await
            .map_err(|err| Error::can_not_receive_request_result(err))?;

        match result {
            AppRequestResult::Error { text } => Err(Error::app_request_error(&text)),
            AppRequestResult::Ok { result } => serde_json::from_value(result)
                .map_err(|err| Error::can_not_parse_request_result(err)),
        }
    }
}

pub(crate) struct AppObject<P: Serialize, R: DeserializeOwned> {
    context: Arc<DengineContext>,
    object_handler: Arc<Request>,
    phantom: std::marker::PhantomData<(P, R)>,
}

impl<P, R> AppObject<P, R>
where
    P: Serialize,
    R: DeserializeOwned,
{
    pub fn new(context: Arc<DengineContext>, object_handler: Arc<Request>) -> AppObject<P, R> {
        AppObject {
            context,
            object_handler,
            phantom: std::marker::PhantomData,
        }
    }

    pub async fn call(&self, params: P) -> ClientResult<R> {
        self.context.app_request(&self.object_handler, params).await
    }

    pub fn notify(&self, params: P) {
        self.object_handler
            .response(params, ResponseType::AppNotify as u32)
    }
}

#[derive(Serialize, Deserialize, ApiType, Default, Clone)]
pub struct ParamsOfAppRequest {
    /// Request ID. Should be used in `resolve_app_request` call
    pub app_request_id: u32,
    /// Request describing data
    pub request_data: serde_json::Value,
}

#[derive(Serialize, Deserialize, ApiType, Clone)]
#[serde(tag = "type")]
pub enum AppRequestResult {
    /// Error occurred during request processing
    Error {
        /// Error description
        text: String,
    },
    /// Request processed successfully
    Ok {
        /// Request processing result
        result: serde_json::Value,
    },
}

impl Default for AppRequestResult {
    fn default() -> Self {
        AppRequestResult::Error {
            text: Default::default(),
        }
    }
}

#[derive(Serialize, Deserialize, ApiType, Default, Clone)]
pub struct ParamsOfResolveAppRequest {
    /// Request ID received from SDK
    pub app_request_id: u32,
    /// Result of request processing
    pub result: AppRequestResult,
}

/// Resolves application request processing result
#[api_function]
pub async fn resolve_app_request(
    context: Arc<DengineContext>,
    params: ParamsOfResolveAppRequest,
) -> ClientResult<()> {
    let request_id = params.app_request_id;
    let sender = context
        .app_requests
        .lock()
        .await
        .remove(&request_id)
        .ok_or(Error::no_such_request(request_id))?;

    sender
        .send(params.result)
        .map_err(|_| Error::can_not_send_request_result(request_id))
}

#[derive(ApiType, Default, Serialize, Deserialize)]
pub struct ResultOfGetApiReference {
    pub api: API,
}

/// Returns Core Library API reference
#[api_function]
pub fn get_api_reference(_context: Arc<DengineContext>) -> ClientResult<ResultOfGetApiReference> {
    Ok(ResultOfGetApiReference {
        api: super::runtime::Runtime::api().clone(),
    })
}