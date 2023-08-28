use super::callbacks;
use super::interfaces::{
    SigningBoxInput, EncryptionBoxInput, Terminal, Echo
};
use super::config::Config;
use super::helpers::TonClient;
use num_bigint::BigInt;
use num_traits::cast::NumCast;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use dengine::prelude::{DebotInterface, DebotInterfaceExecutor, BrowserCallbacks, BrowserRef};
use ton_client::encoding::{decode_abi_bigint, decode_abi_number};
use ton_client::crypto::{KeyPair};

pub struct SupportedInterfaces {
    client: TonClient,
    interfaces: HashMap<String, Arc<dyn DebotInterface + Send + Sync>>,
    browser: BrowserRef,
}

#[async_trait::async_trait]
impl DebotInterfaceExecutor for SupportedInterfaces {
    fn get_interfaces(&self) -> &HashMap<String, Arc<dyn DebotInterface + Send + Sync>> {
        &self.interfaces
    }
    fn get_client(&self) -> TonClient {
        self.client.clone()
    }
    fn get_browser(&self) -> BrowserRef {
        self.browser.clone()
    }
}

impl SupportedInterfaces {
    pub fn new(client: TonClient, debot_key: KeyPair, browser: BrowserRef) -> Self {
        let mut interfaces = HashMap::new();

        let iface: Arc<dyn DebotInterface + Send + Sync> = Arc::new(Echo::new());
        interfaces.insert(iface.get_id(), iface);

        let iface: Arc<dyn DebotInterface + Send + Sync> = Arc::new(Terminal::new(browser.clone()));
        interfaces.insert(iface.get_id(), iface);

        let iface: Arc<dyn DebotInterface + Send + Sync> = Arc::new(SigningBoxInput::new(client.clone(), debot_key.clone()));
        interfaces.insert(iface.get_id(), iface);

        let iface: Arc<dyn DebotInterface + Send + Sync> = Arc::new(EncryptionBoxInput::new(client.clone(), debot_key));
        interfaces.insert(iface.get_id(), iface);

        Self { client, interfaces, browser }
    }
}

pub fn decode_answer_id(args: &Value) -> Result<u32, String> {
    u32::from_str_radix(
        args["answerId"]
            .as_str()
            .ok_or("answer id not found in argument list".to_string())?,
        10,
    )
    .map_err(|e| format!("{}", e))
}

pub fn decode_arg(args: &Value, name: &str) -> Result<String, String> {
    args[name]
        .as_str()
        .ok_or(format!("\"{}\" not found", name))
        .map(|x| x.to_string())
}

pub fn decode_bool_arg(args: &Value, name: &str) -> Result<bool, String> {
    args[name]
        .as_bool()
        .ok_or(format!("\"{}\" not found", name))
}

pub fn decode_string_arg(args: &Value, name: &str) -> Result<String, String> {
    decode_arg(args, name)
}

pub fn decode_nonce(args: &Value) -> Result<String, String> {
    decode_arg(args, "nonce")
}

pub fn decode_prompt(args: &Value) -> Result<String, String> {
    decode_string_arg(args, "prompt")
}

pub fn decode_num_arg<T>(args: &Value, name: &str) -> Result<T, String>
where
    T: NumCast,
{
    let num_str = decode_arg(args, name)?;
    decode_abi_number::<T>(&num_str)
        .map_err(|e| format!("failed to parse integer \"{}\": {}", num_str, e))
}

pub fn decode_int256(args: &Value, name: &str) -> Result<BigInt, String> {
    let num_str = decode_arg(args, name)?;
    decode_abi_bigint(&num_str)
        .map_err(|e| format!("failed to decode integer \"{}\": {}", num_str, e))
}

pub fn decode_array<F, T>(args: &Value, name: &str, validator: F) -> Result<Vec<T>, String>
where
    F: Fn(&Value) -> Option<T>,
{
    let array = args[name]
        .as_array()
        .ok_or(format!("\"{}\" is invalid: must be array", name))?;
    let mut strings = vec![];
    for elem in array {
        strings.push(validator(elem).ok_or("invalid array element type".to_string())?);
    }
    Ok(strings)
}
