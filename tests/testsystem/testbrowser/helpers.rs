use std::env;
use std::path::{PathBuf, Path};
use super::config::{Config, LOCALNET};

use std::sync::Arc;
use std::time::{Duration, SystemTime};
use ton_client::abi::{
    Abi, AbiConfig, AbiContract, DecodedMessageBody, DeploySet, ParamsOfDecodeMessageBody,
    ParamsOfEncodeMessage, Signer,
};
use ton_client::crypto::{CryptoConfig, KeyPair};
use ton_client::error::ClientError;
use ton_client::net::{query_collection, OrderBy, ParamsOfQueryCollection, NetworkConfig};
use ton_client::{ClientConfig, ClientContext};
use ton_block::{MsgAddressInt};
use std::str::FromStr;
use serde_json::{Value, json};
use ton_client::abi::Abi::Contract;
use super::config::{resolve_net_name};

pub const HD_PATH: &str = "m/44'/396'/0'/0/0";
pub const WORD_COUNT: u8 = 12;

pub const SDK_EXECUTION_ERROR_CODE: u32 = 414;
const CONFIG_BASE_NAME: &str = "tonos-cli.conf.json";
const GLOBAL_CONFIG_PATH: &str = ".tonos-cli.global.conf.json";

pub fn default_config_name() -> String {
    env::current_dir()
        .map(|dir| {
            dir.join(PathBuf::from(CONFIG_BASE_NAME)).to_str().unwrap().to_string()
        })
        .unwrap_or(CONFIG_BASE_NAME.to_string())
}

pub fn global_config_path() -> String {
    env::current_exe()
        .map(|mut dir| {
            dir.set_file_name(GLOBAL_CONFIG_PATH);
            dir.to_str().unwrap().to_string()
        })
        .unwrap_or(GLOBAL_CONFIG_PATH.to_string())
}

pub fn read_keys(filename: &str) -> Result<KeyPair, String> {
    let keys_str = std::fs::read_to_string(filename)
        .map_err(|e| format!("failed to read the keypair file: {}", e))?;
    let keys: KeyPair = serde_json::from_str(&keys_str)
        .map_err(|e| format!("failed to load keypair: {}", e))?;
    Ok(keys)
}

pub fn load_ton_address(addr: &str, config: &Config) -> Result<String, String> {
    let addr = if addr.find(':').is_none() {
        format!("{}:{}", config.wc, addr)
    } else {
        addr.to_owned()
    };
    let _ = MsgAddressInt::from_str(&addr)
        .map_err(|e| format!("Address is specified in the wrong format. Error description: {}", e))?;
    Ok(addr)
}

pub fn now() -> Result<u32, String> {
    Ok(SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_err(|e| format!("failed to obtain system time: {}", e))?
        .as_secs() as u32
    )
}

pub fn now_ms() -> u64 {
    chrono::prelude::Utc::now().timestamp_millis() as u64
}

pub type TonClient = Arc<ClientContext>;

pub fn create_client_local() -> Result<TonClient, String> {
    let cli = ClientContext::new(ClientConfig::default())
        .map_err(|e| format!("failed to create tonclient: {}", e))?;
    Ok(Arc::new(cli))
}

pub fn get_server_endpoints(config: &Config) -> Vec<String> {
    let mut cur_endpoints = match config.endpoints.len() {
        0 => vec![config.url.clone()],
        _ => config.endpoints.clone(),
    };
    cur_endpoints.iter_mut().map(|end| {
            let mut end = end.trim_end_matches('/').to_owned();
        if config.project_id.is_some() {
            end.push_str("/");
            end.push_str(&config.project_id.clone().unwrap());
        }
        end.to_owned()
    }).collect::<Vec<String>>()
}

pub fn create_client(config: &Config) -> Result<TonClient, String> {
    let modified_endpoints = get_server_endpoints(config);
    let endpoints_cnt = if resolve_net_name(&config.url).unwrap_or(config.url.clone()).eq(LOCALNET) {
        1_u8
    } else {
        modified_endpoints.len() as u8
    };
    let cli_conf = ClientConfig {
        abi: AbiConfig {
            workchain: config.wc,
            message_expiration_timeout: config.lifetime * 1000,
            message_expiration_timeout_grow_factor: 1.3,
        },
        crypto: CryptoConfig {
            mnemonic_dictionary: ton_client::crypto::MnemonicDictionary::English,
            mnemonic_word_count: WORD_COUNT,
            hdkey_derivation_path: HD_PATH.to_string(),
        },
        network: NetworkConfig {
            server_address: Some(config.url.to_owned()),
            sending_endpoint_count: endpoints_cnt,
            endpoints: if modified_endpoints.is_empty() {
                    None
                } else {
                    Some(modified_endpoints)
                },
            message_retries_count: config.retries as i8,
            message_processing_timeout: 30000,
            wait_for_timeout: config.timeout,
            out_of_sync_threshold: Some(config.out_of_sync_threshold * 1000),
            access_key: config.access_key.clone(),
            ..Default::default()
        },
        ..Default::default()
    };
    let cli =
        ClientContext::new(cli_conf).map_err(|e| format!("failed to create tonclient: {}", e))?;
    Ok(Arc::new(cli))
}

pub async fn query_raw(
    config: &Config,
    collection: &str,
    filter: Option<&str>,
    limit: Option<&str>,
    order: Option<&str>,
    result: &str
) -> Result<(), String>
{
    let context = create_client(config)?;

    let filter = filter.map(|s| serde_json::from_str(s)).transpose()
        .map_err(|e| format!("Failed to parse filter field: {}", e))?;
    let limit = limit.map(|s| s.parse::<u32>()).transpose()
        .map_err(|e| format!("Failed to parse limit field: {}", e))?;
    let order = order.map(|s| serde_json::from_str(s)).transpose()
        .map_err(|e| format!("Failed to parse order field: {}", e))?;

    let query = ton_client::net::query_collection(
        context.clone(),
        ParamsOfQueryCollection {
            collection: collection.to_owned(),
            filter,
            limit,
            order,
            result: result.to_owned(),
            ..Default::default()
        }
    ).await.map_err(|e| format!("Failed to execute query: {}", e))?;

    println!("{:#}", Value::Array(query.result));
    Ok(())
}

pub async fn query_with_limit(
    ton: TonClient,
    collection: &str,
    filter: Value,
    result: &str,
    order: Option<Vec<OrderBy>>,
    limit: Option<u32>,
) -> Result<Vec<Value>, ClientError> {
    query_collection(
        ton,
        ParamsOfQueryCollection {
            collection: collection.to_owned(),
            filter: Some(filter),
            result: result.to_owned(),
            order,
            limit,
            ..Default::default()
        },
    )
        .await
        .map(|r| r.result)
}

pub async fn query_message(
    ton: TonClient,
    message_id: &str,
) -> Result<String, String> {
    let messages = query_with_limit(
        ton.clone(),
        "messages",
        json!({ "id": { "eq": message_id } }),
        "boc",
        None,
        Some(1),
    ).await
        .map_err(|e| format!("failed to query account data: {}", e))?;
    if messages.is_empty() {
        Err("message with specified id was not found.".to_string())
    }
    else {
        Ok(messages[0]["boc"].as_str().ok_or("Failed to obtain message boc.".to_string())?.to_string())
    }
}

pub async fn query_account_field(ton: TonClient, address: &str, field: &str) -> Result<String, String> {
    let accounts = query_with_limit(
        ton.clone(),
        "accounts",
        json!({ "id": { "eq": address } }),
        field,
        None,
        Some(1),
    ).await
        .map_err(|e| format!("failed to query account data: {}", e))?;
    if accounts.is_empty() {
        return Err(format!("account with address {} not found", address));
    }
    let data = accounts[0][field].as_str();
    if data.is_none() {
        return Err(format!("account doesn't contain {}", field));
    }
    Ok(data.unwrap().to_string())
}


pub async fn decode_msg_body(
    ton: TonClient,
    abi_path: &str,
    body: &str,
    is_internal: bool,
    _config: &Config,
) -> Result<DecodedMessageBody, String> {

    let abi = load_abi(abi_path)?;
    ton_client::abi::decode_message_body(
        ton,
        ParamsOfDecodeMessageBody {
            abi,
            body: body.to_owned(),
            is_internal,
            ..Default::default()
        },
    )
    .map_err(|e| format!("failed to decode body: {}", e))
}

fn load_abi_str<P: AsRef<Path>>(abi_path: P) -> Result<String, String> {
    Ok(std::fs::read_to_string(abi_path)
        .map_err(|e| format!("failed to read ABI file: {}", e))?)
}

pub fn load_abi<P: AsRef<Path>>(abi_path: P) -> Result<Abi, String> {
    let abi_str = load_abi_str(abi_path)?;
    Ok(Contract(serde_json::from_str::<AbiContract>(&abi_str)
            .map_err(|e| format!("ABI is not a valid json: {}", e))?,
    ))
}

pub fn load_tvc<P: AsRef<Path>>(tvc_path: P) -> Result<String, String> {
    let tvc_vec = std::fs::read(tvc_path)
        .map_err(|e| format!("failed to read TVC file: {}", e))?;
    Ok(base64::encode(&tvc_vec))
}

pub async fn calc_acc_address(
    ton: TonClient,
    tvc: String,
    pubkey: Option<String>,
    init_data_str: Option<&str>,
    abi: Abi,
) -> Result<String, String> {
    let initial_data = init_data_str
        .map(serde_json::from_str)
        .transpose()
        .map_err(|e| format!("initial data is not in json: {}", e))?;

    let dset = DeploySet {
        tvc: Some(tvc),
        initial_data,
        initial_pubkey: pubkey.clone(),
        ..Default::default()
    };
    let result = ton_client::abi::encode_message(
        ton,
        ParamsOfEncodeMessage {
            abi,
            deploy_set: Some(dset),
            signer: pubkey.map(|public_key| Signer::External {
                public_key,
            }).unwrap_or(Signer::None),
            ..Default::default()
        },
    )
    .await
    .map_err(|e| format!("cannot generate address: {}", e))?;
    Ok(result.address)
}

pub fn answer_filter(depool: &str, wallet: &str, since: u32) -> serde_json::Value {
    json!({
        "src": { "eq": depool },
        "dst": { "eq": wallet },
        "created_at": {"ge": since }
    })
}

pub fn events_filter(addr: &str, since: u32) -> serde_json::Value {
    json!({
        "src": { "eq": addr },
        "msg_type": {"eq": 2 },
        "created_at": {"ge": since }
    })
}

pub async fn print_message(ton: TonClient, message: &Value, abi: &str, is_internal: bool) -> Result<(String, String), String> {
    println!("Id: {}", message["id"].as_str().unwrap_or("Undefined"));
    let value = message["value"].as_str().unwrap_or("0x0");
    let value = u64::from_str_radix(value.trim_start_matches("0x"), 16)
        .map_err(|e| format!("failed to decode msg value: {}", e))?;
    let value: f64 = value as f64 / 1e9;
    println!("Value: {:.9}", value);
    println!("Created at: {} ({})",
        message["created_at"].as_u64().unwrap_or(0),
        message["created_at_string"].as_str().unwrap_or("Undefined")
    );

    let body = message["body"].as_str();
    if body.is_some() {
        let body = body.unwrap();
        let result = ton_client::abi::decode_message_body(
            ton.clone(),
            ParamsOfDecodeMessageBody {
                abi: load_abi(abi)?,
                body: body.to_owned(),
                is_internal,
                ..Default::default()
            },
        );
        let (name, args) = if result.is_err() {
            ("unknown".to_owned(), "{}".to_owned())
        } else {
            let result = result.unwrap();
            (result.name, serde_json::to_string(&result.value)
                .map_err(|e| format!("failed to serialize the result: {}", e))?)
        };
        println!("Decoded body:\n{} {}\n", name, args);
        return Ok((name, args));
    }
    println!();
    Ok(("".to_owned(), "".to_owned()))
}


pub fn check_dir(path: &str) -> Result<(), String> {
    if !path.is_empty() && !std::path::Path::new(path).exists() {
        std::fs::create_dir(path)
            .map_err(|e| format!("Failed to create folder {}: {}", path, e))?;
    }
    Ok(())
}