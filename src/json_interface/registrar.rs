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

use super::handlers::{
    CallHandler, CallNoArgsHandler, SpawnHandler, SpawnHandlerAppObject, SpawnNoArgsHandler,
};
use super::runtime::RuntimeHandlers;
use super::client::{AppObject, DengineContext};
use ton_client::error::ClientResult;
use api_info::{ApiModule, ApiType, Module};
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::future::Future;
use std::sync::Arc;

pub(crate) struct ModuleReg<'h> {
    handlers: &'h mut RuntimeHandlers,
    module: Module,
}

impl<'h> ModuleReg<'h> {
    pub fn new<M: ApiModule>(handlers: &'h mut RuntimeHandlers) -> Self {
        Self {
            module: M::api(),
            handlers,
        }
    }

    pub fn register(self) {
        self.handlers.add_module(self.module);
    }

    pub fn register_error_code<T: ApiType>(&mut self) {
        let mut ty = T::api();
        ty.name = format!(
            "{}{}{}",
            self.module.name[0..1].to_uppercase(),
            self.module.name[1..].to_lowercase(),
            ty.name
        );
        self.module.types.push(ty);
    }

    pub fn register_type<T: ApiType>(&mut self) {
        let ty = T::api();
        if let api_info::Type::None = ty.value {
            if ty.name == "unit" {
                return;
            }
        }
        if self
            .module
            .types
            .iter()
            .find(|x| x.name == ty.name)
            .is_none()
        {
            self.module.types.push(ty);
        }
    }

    pub fn register_async_fn<P, R, F>(
        &mut self,
        handler: fn(context: Arc<DengineContext>, params: P) -> F,
        api: fn() -> api_info::Function,
    ) where
        P: ApiType + Send + DeserializeOwned + Default + 'static,
        R: ApiType + Send + Serialize + 'static,
        F: Send + Future<Output = ClientResult<R>> + 'static,
    {
        self.register_type::<P>();
        self.register_type::<R>();
        let function = api();
        let name = format!("{}.{}", self.module.name, function.name);
        self.module.functions.push(function);

        self.handlers
            .register_async(name.clone(), Box::new(SpawnHandler::new(handler)));
    }

    pub fn register_async_fn_with_app_object<P, R, F, AP, AR>(
        &mut self,
        handler: fn(context: Arc<DengineContext>, params: P, app_object: AppObject<AP, AR>) -> F,
        api: fn() -> api_info::Function,
    ) where
        P: ApiType + Send + DeserializeOwned + Default + 'static,
        R: ApiType + Send + Serialize + 'static,
        AP: ApiType + Send + Serialize + 'static,
        AR: ApiType + Send + DeserializeOwned + 'static,
        F: Send + Future<Output = ClientResult<R>> + 'static,
    {
        self.register_type::<P>();
        self.register_type::<R>();
        self.register_type::<AP>();
        self.register_type::<AR>();
        let function = api();
        let name = format!("{}.{}", self.module.name, function.name);
        self.module.functions.push(function);
        self.handlers
            .register_async(name.clone(), Box::new(SpawnHandlerAppObject::new(handler)));
    }

    pub fn register_sync_fn<P, R>(
        &mut self,
        handler: fn(context: Arc<DengineContext>, params: P) -> ClientResult<R>,
        api: fn() -> api_info::Function,
    ) where
        P: ApiType + Send + DeserializeOwned + Default + 'static,
        R: ApiType + Send + Serialize + 'static,
    {
        self.register_type::<P>();
        self.register_type::<R>();
        let function = api();
        let name = format!("{}.{}", self.module.name, function.name);
        self.module.functions.push(function);

        self.handlers
            .register_sync(name.clone(), Box::new(CallHandler::new(handler)));

        self.handlers.register_async(
            name.clone(),
            Box::new(SpawnHandler::new(move |context, params| async move {
                handler(context, params)
            })),
        );
    }

    pub fn register_sync_fn_without_args<R>(
        &mut self,
        handler: fn(context: Arc<DengineContext>) -> ClientResult<R>,
        api: fn() -> api_info::Function,
    ) where
        R: ApiType + Send + Serialize + 'static,
    {
        self.register_type::<R>();
        let function = api();
        let name = format!("{}.{}", self.module.name, function.name);
        self.module.functions.push(function);

        self.handlers
            .register_sync(name.clone(), Box::new(CallNoArgsHandler::new(handler)));

        self.handlers.register_async(
            name.clone(),
            Box::new(SpawnNoArgsHandler::new(move |context| async move {
                handler(context)
            })),
        );
    }
}
