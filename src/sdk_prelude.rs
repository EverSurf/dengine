pub use std::sync::Arc;
pub use ton_block::{Message, MsgAddressExt, Serializable};
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
    nacl_box_keypair_from_secret_key, nacl_box_keypair_from_secret_key, nacl_box_open,
    nacl_sign_keypair_from_secret_key, remove_signing_box, remove_signing_box,
    signing_box_get_public_key, signing_box_sign, signing_box_sign, EncryptionBoxHandle,
    EncryptionBoxInfo, KeyPair, ParamsOfChaCha20, ParamsOfEncryptionBoxDecrypt,
    ParamsOfEncryptionBoxEncrypt, ParamsOfEncryptionBoxGetInfo, ParamsOfGenerateRandomBytes,
    ParamsOfHDKeyDeriveFromXPrv, ParamsOfHDKeyDeriveFromXPrvPath, ParamsOfHDKeyPublicFromXPrv,
    ParamsOfHDKeySecretFromXPrv, ParamsOfHDKeyXPrvFromMnemonic, ParamsOfMnemonicDeriveSignKeys,
    ParamsOfMnemonicFromRandom, ParamsOfMnemonicVerify, ParamsOfNaclBox, ParamsOfNaclBox,
    ParamsOfNaclBoxKeyPairFromSecret, ParamsOfNaclBoxKeyPairFromSecret, ParamsOfNaclBoxOpen,
    ParamsOfNaclSignKeyPairFromSecret, ParamsOfSigningBoxSign,
    RegisteredSigningBox, SigningBoxHandle,
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

pub(crate) use crate::{DebotHandle, JsonValue, TonClient, DEBOT_WC};


pub fn abi_to_json_string(obj: &Abi) -> ClientResult<String> {
    match obj {
        Self::Contract(abi) | Self::Serialized(abi) => {
            Ok(serde_json::to_string(abi).map_err(|err| Error::invalid_abi(err))?)
        }
        Self::Json(abi) => Ok(abi.clone()),
        _ => Err(crate::client::Error::not_implemented(
            "ABI handles are not supported yet",
        )),
    }
}