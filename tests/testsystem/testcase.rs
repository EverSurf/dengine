//use anyhow::Result;
use super::testbrowser::{
    calc_acc_address, load_abi, load_tvc, run_debot_browser, Config, TonClient,
};
use super::TestGiver;
use serde_json::json;
use std::path::PathBuf;
use std::sync::Arc;
use ton_client::abi::{encode_message, Abi, CallSet, DeploySet, ParamsOfEncodeMessage, Signer};
use ton_client::crypto::{generate_random_sign_keys, KeyPair};
use ton_client::net::{query_transaction_tree, ParamsOfQueryTransactionTree};
use ton_client::processing::{
    process_message, send_message, wait_for_transaction, ParamsOfProcessMessage,
    ParamsOfSendMessage,
};

pub struct TestCase {
    name: String,
    client: TonClient,
    abi: Abi,
    tvc: String,
    keys: KeyPair,
    pub addr: String,
    giver: Arc<TestGiver>,
    config: Arc<Config>,
}

impl TestCase {
    pub async fn create(
        debot_name: &str,
        client: TonClient,
        giver: Arc<TestGiver>,
        config: Arc<Config>,
    ) -> TestCase {
        let contracts_path: PathBuf = ["tests", "testsystem", "contracts"].iter().collect();
        let abi = load_abi(&contracts_path.join(&format!("{debot_name}.abi.json"))).unwrap();
        let tvc = load_tvc(contracts_path.join(&format!("{debot_name}.tvc"))).unwrap();
        let keys = generate_random_sign_keys(client.clone()).unwrap();
        let addr = calc_acc_address(
            client.clone(),
            tvc.clone(),
            Some(keys.public.clone()),
            None,
            abi.clone(),
        )
        .await
        .unwrap();

        TestCase {
            name: debot_name.to_string(),
            client,
            abi,
            tvc,
            keys,
            addr,
            giver,
            config,
        }
    }

    pub async fn deploy(&self) {
        self.deploy_with_args(json!({})).await;
    }

    pub async fn deploy_with_args(&self, args: serde_json::Value) {
        self.giver.send(self.addr.clone()).await;
        let _ = self
            .call_and_wait_all(
                "constructor",
                args,
                DeploySet::some_with_tvc(Some(self.tvc.clone())),
            )
            .await;
        let args = if let Abi::Contract(abi) = &self.abi {
            if let Some(ver) = &abi.version {
                if ver.split('.').collect::<Vec<&str>>()[1] >= "2" {
                    json!({ "dabi": serde_json::to_string(abi).unwrap() })
                } else {
                    json!({ "dabi": hex::encode(serde_json::to_string(abi).unwrap()) })
                }
            } else {
                json!({ "dabi": hex::encode(serde_json::to_string(abi).unwrap()) })
            }
        } else {
            unreachable!();
        };
        let _ = self.call_and_wait_all("setABI", args, None).await;
    }

    pub async fn call(&self, method: &str, args: serde_json::Value) -> Option<serde_json::Value> {
        self.call_and_wait_all(method, args, None).await
    }

    pub async fn set_icon(&self) {
        let icon_path: PathBuf = ["tests", "testsystem", "contracts"].iter().collect();
        let icon_bytes = std::fs::read(icon_path.join(format!("{}.png", self.name))).unwrap();
        let icon_arg = format!("data:image/png;base64,{}", base64::encode(&icon_bytes));
        let _ = self
            .call_and_wait_all("setIcon", json!({"icon": hex::encode(&icon_arg)}), None)
            .await;
    }

    pub async fn run(self) -> (Option<serde_json::Value>, Vec<String>) {
        run_debot_browser(self.client.clone(), &self.addr, self.config, self.keys)
            .await
            .unwrap()
    }

    async fn call_and_wait_all(
        &self,
        method: &str,
        args: serde_json::Value,
        deploy_set: Option<DeploySet>,
    ) -> Option<serde_json::Value> {
        let params = ParamsOfEncodeMessage {
            abi: self.abi.clone(),
            signer: Signer::Keys {
                keys: self.keys.clone(),
            },
            address: Some(self.addr.clone()),
            call_set: CallSet::some_with_function_and_input(method, args),
            deploy_set,
            ..Default::default()
        };
        let encode_res = encode_message(self.client.clone(), params)
            .await
            .unwrap();
        let (message, in_msg) = (encode_res.message, encode_res.message_id);
        let send_res = send_message(
            self.client.clone(),
            ParamsOfSendMessage {
                message: message.clone(),
                ..Default::default()
            },
            |_| async {},
        )
        .await
        .unwrap();

        let process_res = wait_for_transaction(
            self.client.clone(),
            ton_client::processing::ParamsOfWaitForTransaction {
                shard_block_id: send_res.shard_block_id,
                sending_endpoints: Some(send_res.sending_endpoints),
                message,
                ..Default::default()
            },
            |_| async {},
        )
        .await
        .unwrap();

        query_transaction_tree(
            self.client.clone(),
            ParamsOfQueryTransactionTree {
                in_msg,
                ..Default::default()
            },
        )
        .await
        .unwrap();

        process_res.decoded.and_then(|d| d.output)
    }
}
