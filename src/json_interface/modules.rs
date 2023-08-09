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

use super::registrar::ModuleReg;
use super::runtime::RuntimeHandlers;
use api_derive::{ApiModule, ApiType};
use crate::prelude::{DebotActivity, DebotInfo, Spending};
use crate::bridge_api::{DebotHandle, start_api, fetch_api, send_api, remove_api};

/// Provides information about library.
#[derive(ApiModule)]
#[api_module(name = "client")]
pub(crate) struct ClientModule;

fn register_client(handlers: &mut RuntimeHandlers) {
    let mut module = ModuleReg::new::<ClientModule>(handlers);
    module.register_error_code::<crate::errors::ErrorCode>();
    module.register_type::<ton_client::error::ClientError>();
    module.register_type::<super::client::BindingConfig>();
    module.register_type::<super::client::ParamsOfAppRequest>();
    module.register_type::<super::client::AppRequestResult>();

    module.register_sync_fn_without_args(
        crate::json_interface::client::get_api_reference,
        crate::json_interface::client::get_api_reference_api,
    );
    //module.register_sync_fn_without_args(crate::client::version, crate::client::version_api);
    //module.register_sync_fn_without_args(crate::client::config, crate::client::config_api);
    //module.register_sync_fn_without_args(crate::client::build_info, crate::client::build_info_api);
    module.register_async_fn(
        crate::json_interface::client::resolve_app_request,
        crate::json_interface::client::resolve_app_request_api,
    );
    module.register();
}

/// [UNSTABLE](UNSTABLE.md) [DEPRECATED](DEPRECATED.md) Module for working with debot.
#[derive(ApiModule)]
#[api_module(name = "debot")]
pub struct DebotModule;

fn register_debot(handlers: &mut RuntimeHandlers) {
    let mut module = ModuleReg::new::<DebotModule>(handlers);
    module.register_error_code::<crate::errors::ErrorCode>();
    module.register_type::<DebotHandle>();
    module.register_type::<DebotInfo>();
    module.register_type::<DebotActivity>();
    module.register_type::<Spending>();
    module.register_async_fn_with_app_object(
        super::debot::init,
        super::debot::init_api,
    );
    module.register_async_fn(crate::start, start_api);
    module.register_async_fn(crate::fetch, fetch_api);
    module.register_async_fn(crate::send, send_api);
    module.register_sync_fn(crate::remove, remove_api);
    module.register();
}

pub(crate) fn register_modules(handlers: &mut RuntimeHandlers) {
    register_client(handlers);
    register_debot(handlers);
}
