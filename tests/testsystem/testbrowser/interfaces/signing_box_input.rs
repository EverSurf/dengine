use super::super::dinterface::{decode_answer_id, decode_prompt, decode_array};
use super::super::helpers::TonClient;

use serde_json::Value;
use ton_client::abi::Abi;
use ton_client::crypto::{get_signing_box, KeyPair};
use dengine::prelude::{DebotInterface, InterfaceResult};
use ton_client::encoding::decode_abi_bigint;
use serde_json::json;

pub const ID: &str = "c13024e101c95e71afb1f5fa6d72f633d51e721de0320d73dfd6121a54e4d40a";

const ABI: &str = r#"
{
    "ABI version": 2,
    "version": "2.2",
    "header": ["time"],
    "functions": [
        {
            "name": "get",
            "id": "0x04895be9",
            "inputs": [
                {"name":"answerId","type":"uint32"},
                {"name":"prompt","type":"string"},
                {"name":"possiblePublicKeys","type":"uint256[]"}
            ],
            "outputs": [
                {"name":"handle","type":"uint32"}
            ]
        },
        {
            "name": "constructor",
            "id": "0x68b55f3f",
            "inputs": [
            ],
            "outputs": [
            ]
        }
    ],
    "data": [
    ],
    "events": [
    ],
    "fields": [
        {"name":"_pubkey","type":"uint256"},
        {"name":"_timestamp","type":"uint64"},
        {"name":"_constructorFlag","type":"bool"}
    ]
}
"#;

pub struct SigningBoxInput {
    client: TonClient,
    keypair: KeyPair,
}
impl SigningBoxInput {
    pub fn new(client: TonClient, keypair: KeyPair) -> Self {
        Self { client, keypair}
    }

    async fn get(&self, args: &Value) -> InterfaceResult {
        let answer_id = decode_answer_id(args)?;
        let prompt = decode_prompt(args)?;
        let _possible_keys = decode_array(
            args,
            "possiblePublicKeys",
            |elem| {
                decode_abi_bigint(elem.as_str()?).ok()?;
                Some(elem.as_str().unwrap().to_string())
            }
        )?;
        println!("{}", prompt);
        let handle = get_signing_box(self.client.clone(), self.keypair.clone())
            .await
            .map(|r| r.handle)
            .map_err(|e| e.to_string())?;
        Ok((answer_id, json!({ "handle": handle}) ))
    }
}

#[async_trait::async_trait]
impl DebotInterface for SigningBoxInput {
    fn get_id(&self) -> String {
        ID.to_string()
    }

    fn get_abi(&self) -> Abi {
        Abi::Json(ABI.to_owned())
    }

    async fn call(&self, func: &str, args: &Value) -> InterfaceResult {
        match func {
            "get" => self.get(args).await,
            _ => Err(format!("function \"{}\" is not implemented", func)),
        }
    }
}