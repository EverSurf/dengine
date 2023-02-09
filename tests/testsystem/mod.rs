mod testcase;
mod testbrowser;

use serde::Deserialize;
pub use testcase::TestCase;
use ton_client::abi::{Abi, Signer, CallSet, AbiContract, ParamsOfEncodeMessage};
use ton_client::processing::{process_message, ParamsOfProcessMessage};
use ton_client::crypto::{KeyPair};
use testbrowser::{TonClient, Config, create_client};
use serde_json::json;

#[derive(Deserialize)]
struct GiverParams {
    addr: String,
    abi_path: String,
    keys_path: String,
}

#[derive(Deserialize)]
struct GiverAddresses {
    dev: GiverParams,
    se: GiverParams,
}

#[derive(Clone)]
pub struct TestGiver {
    params: ParamsOfEncodeMessage,
    client: TonClient,
}

impl TestGiver {
    pub fn new(client: TonClient, evers: u64) -> Self {
        
        let mut givers = serde_json::from_str::<GiverAddresses>(&std::fs::read_to_string("givers/addresses.json").unwrap()).unwrap();

        let address = Some(givers.se.addr);
        let abi = givers.se.abi_path;
        let abi = std::fs::read_to_string(format!("givers/{abi}")).unwrap();
        let abi = Abi::Contract(serde_json::from_str::<AbiContract>(&abi).unwrap());
        let keys = givers.se.keys_path;
        let keys = std::fs::read_to_string(format!("givers/{keys}")).unwrap();
        let keys = serde_json::from_str::<KeyPair>(&keys).unwrap();
        
        let params = ParamsOfEncodeMessage {
            abi,
            signer: Signer::Keys { keys },
            address,
            call_set: Some(CallSet {
                function_name: "sendTransaction".to_string(),
                input: Some(json!({
                    "dest": String::new(),
                    "value": evers * 1000000000,
                    "bounce": false
                })),
                ..Default::default()
            }),
            ..Default::default()
        };

        Self {params, client}
    }

    pub async fn send(&self, dest: String) {
        let mut params = self.params.clone(); 
        if let Some(ref mut call) = params.call_set {
            if let Some(ref mut input) = call.input {
                input["dest"] = json!(dest);
            }
        }
        process_message(
            self.client.clone(),
            ParamsOfProcessMessage {
                    message_encode_params: params,
                    ..Default::default()
            },
            |_| { async {}}
        ).await.unwrap();
    }
}

pub struct TestSystem {
    giver: TestGiver,
    client: TonClient,
    config: Config,
}

impl TestSystem {
    pub fn new(giver_amount: u64) -> Self {
        let config = Config::from_env();
        let client = create_client(&config).unwrap();
        let giver = TestGiver::new(client.clone(), giver_amount);
        Self {
            client,
            config,
            giver,
        }
    }

    pub async fn new_test(&self, name: &str) -> TestCase {
        TestCase::create(
            name,
            self.client.clone(),
            self.giver.clone(),
            self.config.clone(),
        ).await

    }
}