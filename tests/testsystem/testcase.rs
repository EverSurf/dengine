//use anyhow::Result;
use ton_client::crypto::{KeyPair, generate_random_sign_keys};
use ton_client::abi::{Abi, DeploySet, Signer, CallSet, ParamsOfEncodeMessage};
use ton_client::processing::{ParamsOfProcessMessage, process_message};
use super::testbrowser::{TonClient, Config, run_debot_browser, calc_acc_address, load_tvc, load_abi};
use super::TestGiver;
use serde_json::json;

pub struct TestCase {
    client: TonClient,
    abi: Abi,
    tvc: String,
    keys: KeyPair,
    addr: String,
    giver: TestGiver,
    config: Config,
}

impl TestCase {
    pub async fn create(debot_name: &str, client: TonClient, giver: TestGiver, config: Config) -> TestCase {
        let abi = load_abi(&format!("./{debot_name}.abi.json")).unwrap();
        let tvc = load_tvc(&format!("./{debot_name}.tvc")).unwrap();
        let keys = generate_random_sign_keys(client.clone()).unwrap();
        let addr = calc_acc_address(
            client.clone(),
            tvc.clone(), Some(keys.public.clone()), None, abi.clone()
        ).await.unwrap();
        
        TestCase {
            client, abi, tvc, keys, addr, giver, config
        }
    }
    //pub fn load_abi(&mut self) {
    //    
    //}
    //pub fn load_tvc(&mut self) {
    //    
    //}

    pub async fn deploy(&self) {
        let mut params = ParamsOfEncodeMessage {
            abi: self.abi.clone(),
            deploy_set: DeploySet::some_with_tvc(self.tvc.clone()),
            signer: Signer::Keys { keys: self.keys.clone() },
            processing_try_index: None,
            address: Some(self.addr.clone()),
            call_set: CallSet::some_with_function("constructor"),
        };
        self.giver.send(self.addr.clone()).await;
        
        process_message(
            self.client.clone(),
            ParamsOfProcessMessage {
                message_encode_params: params.clone(),
                ..Default::default()
            },
            |_| {async {}},
        ).await.unwrap();

        params.deploy_set = None;
        if let Abi::Contract(ref abi_contract) = self.abi {
            params.call_set = CallSet::some_with_function_and_input(
                "setABI",
                json!({ "dabi": hex::encode(serde_json::to_string(abi_contract).unwrap().as_bytes()) })
            );
        }

        process_message(
            self.client.clone(),
            ParamsOfProcessMessage {
                message_encode_params: params,
                ..Default::default()
            },
            |_| {async {} },
        ).await.unwrap();

    }

    pub async fn run(self) {
        run_debot_browser(&self.addr, self.config, self.keys).await;
    }
}