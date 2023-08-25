use crate::sdk_prelude::*;
pub use chrono::{Local, TimeZone};
//use serde::serde;
use serde_derive::{Deserialize, Serialize};
use serde_json::json;
use crate::browser::BrowserRef;

#[derive(Serialize, Deserialize, Clone)]
pub(super) struct ResultOfGetAccountState {
    pub balance: String,
    pub acc_type: i8,
    last_trans_lt: String,
    #[serde(default)]
    code: String,
    #[serde(default)]
    data: String,
    #[serde(rename(deserialize = "library"))]
    #[serde(default)]
    lib: String,
}

impl Default for ResultOfGetAccountState {
    fn default() -> Self {
        Self {
            balance: string_with_zero(),
            last_trans_lt: string_with_zero(),
            acc_type: -1,
            code: String::new(),
            data: String::new(),
            lib: String::new(),
        }
    }
}

fn string_with_zero() -> String {
    "0".to_string()
}

pub(super) fn format_string(fstr: &str, params: &serde_json::Value) -> String {
    let mut str_builder = String::new();
    for (i, s) in fstr.split("{}").enumerate() {
        str_builder += s;
        str_builder += &format_arg(params, i);
    }
    str_builder
}

pub(super) fn format_arg(params: &serde_json::Value, i: usize) -> String {
    let idx = i.to_string();
    if let Some(arg) = params["param".to_owned() + &idx].as_str() {
        return arg.to_owned();
    }
    if let Some(arg) = params["str".to_owned() + &idx].as_str() {
        return String::from_utf8(hex::decode(arg).unwrap_or_default()).unwrap_or_default();
    }
    if let Some(arg) = params["number".to_owned() + &idx].as_str() {
        // TODO: need to use big number instead of u64
        return format!(
            "{}",
            // TODO: remove unwrap and return error
            decode_abi_number::<u64>(arg).unwrap()
        );
    }
    if let Some(arg) = params["utime".to_owned() + &idx].as_str() {
        let utime = decode_abi_number::<u32>(arg).unwrap();
        return if utime == 0 {
            "undefined".to_owned()
        } else {
            let date = Local.timestamp_opt(utime as i64, 0).unwrap();
            date.to_rfc2822()
        };
    }
    String::new()
}

pub(super) fn generate_random(ton: TonClient, args: &serde_json::Value) -> Result<String, String> {
    let len_str = get_arg(args, "length")?;
    let len = len_str
        .parse::<u32>()
        .map_err(|e| format!("failed to parse length: {e}"))?;
    let result = generate_random_bytes(ton, ParamsOfGenerateRandomBytes { length: len })
        .map_err(|e| format!(" failed to generate random: {e}"))?;
    Ok(result.bytes)
}

fn get_arg(args: &serde_json::Value, name: &str) -> Result<String, String> {
    args[name]
        .as_str()
        .ok_or(format!("\"{name}\" not found"))
        .map(|v| v.to_string())
}

pub(super) async fn get_account_state(
    browser: BrowserRef,
    args: &serde_json::Value,
) -> ResultOfGetAccountState {
    get_account(browser, args)
        .await
        .map(|x| serde_json::from_value(x).unwrap_or_default())
        .unwrap_or_default()
}

pub(super) async fn get_account(
    browser: BrowserRef,
    args: &serde_json::Value,
) -> Result<serde_json::Value, String> {
    let addr = get_arg(args, "addr")?.to_lowercase();
    let mut accounts = browser.query_collection(
        ParamsOfQueryCollection {
            collection: "accounts".to_owned(),
            filter: Some(json!({
                "id": { "eq": addr }
            })),
            result: "boc".to_owned(),
            order: None,
            limit: Some(1),
        },
    )
    .await
    .map_err(|e| format!("account query failed: {e}"))?
    .result;

    if accounts.is_empty() {
        return Err("account not found".to_string());
    }

    let cli = ClientContext::new(ClientConfig::default())
        .map_err(|e| format!("{}", e))?;
    let acc = parse_account(
        Arc::new(cli),
        ParamsOfParse {
            boc: get_arg(&accounts.swap_remove(0), "boc")?,
        },
    )
    .map_err(|e| format!("failed to parse account from boc: {e}"))?
    .parsed;

    Ok(acc)
}
