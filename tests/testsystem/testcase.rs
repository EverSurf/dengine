//use anyhow::Result;
use super::testbrowser::{
    calc_acc_address, load_abi, load_tvc, run_debot_browser, Config, TonClient,
};
use super::TestGiver;
use serde_json::json;
use ton_client::net::{query_transaction_tree, ParamsOfQueryTransactionTree};
use std::path::PathBuf;
use std::sync::Arc;
use ton_client::abi::{Abi, CallSet, DeploySet, ParamsOfEncodeMessage, Signer, encode_message};
use ton_client::crypto::{generate_random_sign_keys, KeyPair};
use ton_client::processing::{process_message, ParamsOfProcessMessage, send_message, ParamsOfSendMessage};

pub struct TestCase {
    client: TonClient,
    abi: Abi,
    tvc: String,
    keys: KeyPair,
    addr: String,
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
        let params = ParamsOfEncodeMessage {
            abi: self.abi.clone(),
            deploy_set: DeploySet::some_with_tvc(Some(self.tvc.clone())),
            signer: Signer::Keys {
                keys: self.keys.clone(),
            },
            address: Some(self.addr.clone()),
            call_set: CallSet::some_with_function("constructor"),
            ..Default::default()
        };
        self.giver.send(self.addr.clone()).await;

        let encode_res = encode_message(self.client.clone(), params).await.unwrap();

        send_message(
            self.client.clone(),
            ParamsOfSendMessage {
                message: encode_res.message,
                ..Default::default()
            },
            |_| async {},
        )
        .await
        .unwrap();

        query_transaction_tree(
            self.client.clone(),
            ParamsOfQueryTransactionTree {
                in_msg: encode_res.message_id,
                ..Default::default()
        }).await.unwrap();
        let input = if let Abi::Contract(abi) = &self.abi {
            json!({ "dabi": serde_json::to_string(abi).unwrap() })
        } else {
            unreachable!();
        };
        let params = ParamsOfEncodeMessage {
            abi: self.abi.clone(),
            signer: Signer::Keys {
                keys: self.keys.clone(),
            },
            address: Some(self.addr.clone()),
            call_set: CallSet::some_with_function_and_input(
                "setABI",
                input,
            ),
            ..Default::default()
        };

        process_message(
            self.client.clone(),
            ParamsOfProcessMessage {
                message_encode_params: params,
                ..Default::default()
            },
            |_| async {},
        )
        .await
        .unwrap();
    }

    pub async fn run(self) -> (Option<serde_json::Value>, Vec<String>) {
        run_debot_browser(self.client.clone(), &self.addr, self.config, self.keys)
            .await
            .unwrap()
    }
}
