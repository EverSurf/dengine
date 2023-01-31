pub(crate) use crate::{JsonValue, TonClient, DEBOT_WC};
pub use std::sync::Arc;
pub use ton_block::{InternalMessageHeader, MsgAddressInt};
pub use ton_block::{Message, MsgAddressExt};
pub use ton_client::abi::{
    decode_message, decode_message_body, encode_message, encode_message_body, Abi, CallSet,
    DeploySet, ErrorCode as SdkErrorCode, ParamsOfDecodeMessage, ParamsOfDecodeMessageBody,
    ParamsOfEncodeMessage, ParamsOfEncodeMessageBody, Signer,
};
pub use ton_client::boc::{
    get_boc_hash, parse_account, parse_message, ParamsOfGetBocHash, ParamsOfParse,
};
pub use ton_client::crypto::{
    chacha20, encryption_box_decrypt, encryption_box_encrypt, encryption_box_get_info,
    generate_random_bytes, get_signing_box, hdkey_derive_from_xprv, hdkey_derive_from_xprv_path,
    hdkey_public_from_xprv, hdkey_secret_from_xprv, hdkey_xprv_from_mnemonic,
    mnemonic_derive_sign_keys, mnemonic_from_random, mnemonic_verify, nacl_box,
    nacl_box_keypair_from_secret_key, nacl_box_open,
    nacl_sign_keypair_from_secret_key, remove_signing_box,
    signing_box_get_public_key, signing_box_sign, EncryptionBoxHandle,
    EncryptionBoxInfo, KeyPair, ParamsOfChaCha20, ParamsOfEncryptionBoxDecrypt,
    ParamsOfEncryptionBoxEncrypt, ParamsOfEncryptionBoxGetInfo, ParamsOfGenerateRandomBytes,
    ParamsOfHDKeyDeriveFromXPrv, ParamsOfHDKeyDeriveFromXPrvPath, ParamsOfHDKeyPublicFromXPrv,
    ParamsOfHDKeySecretFromXPrv, ParamsOfHDKeyXPrvFromMnemonic, ParamsOfMnemonicDeriveSignKeys,
    ParamsOfMnemonicFromRandom, ParamsOfMnemonicVerify, ParamsOfNaclBox,
    ParamsOfNaclBoxKeyPairFromSecret, ParamsOfNaclBoxOpen,
    ParamsOfNaclSignKeyPairFromSecret, ParamsOfSigningBoxSign, RegisteredSigningBox,
    SigningBoxHandle,
};
pub use ton_client::encoding::{decode_abi_bigint, decode_abi_number};
pub use ton_client::error::{ClientError, ClientResult};
pub use ton_client::net::{
    query, query_collection, query_transaction_tree, wait_for_collection, NetworkConfig, OrderBy,
    ParamsOfQuery, ParamsOfQueryCollection, ParamsOfQueryTransactionTree,
    ParamsOfWaitForCollection, SortDirection,
};
pub use ton_client::processing::{
    process_message, send_message, wait_for_transaction, ParamsOfProcessMessage,
    ParamsOfSendMessage, ParamsOfWaitForTransaction, ProcessingEvent,
};
pub use ton_client::tvm::{
    run_executor, run_tvm, AccountForExecutor, ParamsOfRunExecutor, ParamsOfRunTvm,
};
pub use ton_client::{ClientConfig, ClientContext};
pub use ton_types::{BuilderData, IBitstring, SliceData};
use std::str::FromStr;

use reqwest::{
    Response as HttpResponse,
    header::{ HeaderMap },
    Client as HttpClient,
};

#[derive(Clone)]
pub(crate) enum DeserializedBoc {
    Bytes(Vec<u8>),
}

#[derive(Clone)]
pub(crate) struct DeserializedObject<S: ton_block::Deserializable> {
    pub object: S,
}

pub(crate) fn abi_to_json_string(obj: &Abi) -> ClientResult<String> {
    match obj {
        Abi::Contract(abi) | Abi::Serialized(abi) => {
            Ok(serde_json::to_string(abi).map_err(ton_client::abi::Error::invalid_abi)?)
        }
        Abi::Json(abi) => Ok(abi.clone()),
        _ => Err(ton_client::client::Error::not_implemented("ABI handles are not supported yet")),
    }
}

pub(crate) fn serialize_cell_to_bytes(cell: &ton_types::Cell, name: &str) -> ClientResult<Vec<u8>> {
    ton_types::cells_serialization::serialize_toc(cell)
        .map_err(|err| ton_client::boc::Error::serialization_error(err, name))
}

pub(crate) fn serialize_cell_to_base64(cell: &ton_types::Cell, name: &str) -> ClientResult<String> {
    Ok(base64::encode(&serialize_cell_to_bytes(cell, name)?))
}

pub(crate) fn serialize_object_to_cell<S: ton_block::Serializable>(
    object: &S,
    name: &str,
) -> ClientResult<ton_types::Cell> {
    object
        .serialize()
        .map_err(|err| ton_client::boc::Error::serialization_error(err, name))
}

pub(crate) fn serialize_object_to_base64<S: ton_block::Serializable>(
    object: &S,
    name: &str,
) -> ClientResult<String> {
    let cell = serialize_object_to_cell(object, name)?;
    serialize_cell_to_base64(&cell, name)
}

pub(crate) fn deserialize_object_from_cell<S: ton_block::Deserializable>(
    cell: ton_types::Cell,
    name: &str,
) -> ClientResult<S> {
    let tip = match name {
        "message" => {
            "Please check that you have specified the message's BOC, not body, as a parameter."
        }
        _ => "",
    };
    let tip_full = if !tip.is_empty() {
        format!(".\nTip: {}", tip)
    } else {
        "".to_string()
    };
    S::construct_from_cell(cell).map_err(|err| {
        ton_client::boc::Error::invalid_boc(format!(
            "cannot deserialize {} from BOC: {}{}",
            name, err, tip_full
        ))
    })
}

pub(crate) fn deserialize_cell_from_base64(
    b64: &str,
    name: &str,
) -> ClientResult<(Vec<u8>, ton_types::Cell)> {
    let bytes = base64::decode(&b64)
        .map_err(|err| ton_client::boc::Error::invalid_boc(format!("error decode {} BOC base64: {}", name, err)))?;

    let cell =
        ton_types::deserialize_tree_of_cells(&mut std::io::Cursor::new(&bytes)).map_err(|err| {
            ton_client::boc::Error::invalid_boc(format!("{} BOC deserialization error: {}", name, err))
        })?;

    Ok((bytes, cell))
}

pub(crate) fn deserialize_object_from_base64<S: ton_block::Deserializable>(
    b64: &str,
    name: &str,
) -> ClientResult<DeserializedObject<S>> {
    let (_, cell) = deserialize_cell_from_base64(b64, name)?;
    let object = deserialize_object_from_cell(cell, name)?;

    Ok(DeserializedObject {
        object,
    })
}

pub(crate) async fn deserialize_cell_from_boc(
    _context: &ClientContext,
    boc: &str,
    name: &str,
) -> ClientResult<(DeserializedBoc, ton_types::Cell)> {
    deserialize_cell_from_base64(boc, name)
        .map(|(bytes, cell)| (DeserializedBoc::Bytes(bytes), cell))
}

pub(crate) fn slice_from_cell(cell: ton_types::Cell) -> ClientResult<SliceData> {
    SliceData::load_cell(cell).map_err(ton_client::client::Error::invalid_data)
}

pub(crate) fn account_decode(string: &str) -> ClientResult<MsgAddressInt> {
    match MsgAddressInt::from_str(string) {
        Ok(address) => Ok(address),
        Err(_) if string.len() == 48 => decode_std_base64(string),
        Err(err) => Err(ton_client::client::Error::invalid_address(err, string)),
    }
}

const XMODEM: crc::Crc<u16> = crc::Crc::<u16>::new(&crc::CRC_16_XMODEM);
fn ton_crc16(data: &[u8]) -> u16 {
    XMODEM.checksum(data)
}

fn decode_std_base64(data: &str) -> ClientResult<MsgAddressInt> {
    // conversion from base64url
    let data = data.replace('_', "/").replace('-', "+");

    let vec = base64::decode(&data).map_err(|err| ton_client::client::Error::invalid_address(err, &data))?;

    // check CRC and address tag
    let crc = ton_crc16(&vec[..34]).to_be_bytes();

    if crc != vec[34..36] || vec[0] & 0x3f != 0x11 {
        return Err(ton_client::client::Error::invalid_address("CRC mismatch", &data));
    };

    MsgAddressInt::with_standart(None, vec[1] as i8, SliceData::from_raw(vec[2..34].to_vec(), 256))
        .map_err(|err| ton_client::client::Error::invalid_address(err, &data))
}

pub fn default_message_expiration_timeout() -> u32 {
    40000
}

pub fn default_query_timeout() -> u32 {
    60000
}

use std::convert::TryInto;
pub async fn fetch(
    client: &HttpClient,
    url: &str,
    method_str: &str,
    headers: Option<std::collections::HashMap<String, String>>,
    body: Option<String>,
    timeout_ms: u32,
) -> ClientResult<HttpResponse> {
    //let http_client = ClientBuilder::new()
    //        .cookie_provider(cookies.clone())
    //        .build()
    //        .map_err(|err| ton_client::client::Error::http_client_create_error(err))?;

    let mut request = client
        .request(reqwest::Method::from_str(method_str).unwrap(), url)
        .timeout(std::time::Duration::from_millis(timeout_ms as u64));

    if let Some(headers) = headers {
        let headers: HeaderMap = (&headers).try_into().map_err(ton_client::client::Error::http_request_create_error)?;
        request = request.headers(headers);
    }
    if let Some(body) = body {
        request = request.body(body);
    }

    let response = request
        .send()
        .await
        .map_err(ton_client::client::Error::http_request_send_error)?;

    Ok(response)
        
        //FetchResult {
        //headers: Self::header_map_to_string_map(response.headers()),
        //status: response.status().as_u16(),
        //url: response.url().to_string(),
        //remote_address: response.remote_addr().map(|x| x.to_string()),
        //body: response
        //    .text()
        //    .await
        //    .map_err(|err| Error::http_request_parse_error(err))?,
}