use super::dinterface::{
    decode_answer_id, get_arg, get_array_strings, DebotInterface, InterfaceResult,
};
use crate::sdk_prelude::default_query_timeout;
use crate::sdk_prelude::fetch;
use crate::TonClient;
use serde_json::{json, Value};
use std::collections::HashMap;
use ton_client::abi::Abi;

const ABI: &str = r#"
{
	"ABI version": 2,
	"version": "2.2",
	"header": ["time"],
	"functions": [
		{
			"name": "get",
            "id": "0x74dd3fc1",
			"inputs": [
				{"name":"answerId","type":"uint32"},
				{"name":"url","type":"string"},
				{"name":"headers","type":"string[]"}
			],
			"outputs": [
				{"name":"statusCode","type":"int32"},
				{"name":"retHeaders","type":"string[]"},
				{"name":"content","type":"string"}
			]
		},
		{
			"name": "post",
            "id": "0x766d8212",
			"inputs": [
				{"name":"answerId","type":"uint32"},
				{"name":"url","type":"string"},
				{"name":"headers","type":"string[]"},
				{"name":"body","type":"string"}
			],
			"outputs": [
				{"name":"statusCode","type":"int32"},
				{"name":"retHeaders","type":"string[]"},
				{"name":"content","type":"string"}
			]
		}
	]
}
"#;

const ID: &str = "e38aed5884dc3e4426a87c083faaf4fa08109189fbc0c79281112f52e062d8ee";

pub struct NetworkInterface {
    client: reqwest::Client,
}

impl NetworkInterface {
    pub fn new(_client: TonClient) -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }

    async fn post(&self, args: &Value) -> InterfaceResult {
        let answer_id = decode_answer_id(args)?;
        let url = get_arg(args, "url")?;
        let headers = get_array_strings(args, "headers")?;
        let body = get_arg(args, "body")?;
        let answer = self.send(url, headers, Some(body)).await?;
        Ok((answer_id, answer))
    }

    async fn get(&self, args: &Value) -> InterfaceResult {
        let answer_id = decode_answer_id(args)?;
        let url = get_arg(args, "url")?;
        let headers = get_array_strings(args, "headers")?;
        let answer = self.send(url, headers, None).await?;
        Ok((answer_id, answer))
    }

    async fn send(
        &self,
        url: String,
        headers: Vec<String>,
        body: Option<String>,
    ) -> Result<Value, String> {
        let mut header_map = HashMap::new();
        for h in headers {
            let mut iter = h.split(':');
            let key = iter.next();
            let value = iter.next();
            if key.is_some() && value.is_some() {
                header_map.insert(
                    key.unwrap().trim().to_owned(),
                    value.unwrap().trim().to_owned(),
                );
            }
        }
        let response = fetch(
            &self.client,
            &url,
            if body.is_some() { "POST" } else { "GEt" },
            if !header_map.is_empty() {
                Some(header_map)
            } else {
                None
            },
            body,
            default_query_timeout(),
        )
        .await
        .map_err(|e| format!("{e}"))?;

        let mut ret_headers: Vec<String> = vec![];
        for (k, v) in response.headers().iter() {
            ret_headers.push(format!("{k}: {v:?}"));
        }
        let status = response.status().as_u16();
        let content = response.text().await.map_err(|err| err.to_string());
        Ok(json!({
            "statusCode": status,
            "retHeaders": ret_headers,
            "content": content,
        }))
    }
}

#[async_trait::async_trait]
impl DebotInterface for NetworkInterface {
    fn get_id(&self) -> String {
        ID.to_string()
    }

    fn get_abi(&self) -> Abi {
        Abi::Json(ABI.to_owned())
    }

    async fn call(&self, func: &str, args: &Value) -> InterfaceResult {
        match func {
            "get" => self.get(args).await,
            "post" => self.post(args).await,
            _ => Err(format!("function \"{func}\" is not implemented")),
        }
    }
}