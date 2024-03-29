//use super::term_signing_box::TerminalSigningBox;
use super::config::Config;
use super::helpers::TonClient;
use super::term_browser::{input, terminal_input};
use dengine::prelude::*;
use std::collections::VecDeque;
use std::io;
use std::sync::{Arc, RwLock};
use ton_client::crypto::{ParamsOfEncryptionBoxGetInfo, RegisteredSigningBox, SigningBoxHandle};
use ton_client::error::ClientResult;
use ton_client::processing::ParamsOfSendMessage;

#[derive(Default)]
struct ActiveState {
    state_id: u8,
    active_actions: Vec<DAction>,
    msg_queue: VecDeque<String>,
    outputs: Vec<String>,
}

pub(super) struct Callbacks {
    client: TonClient,
    state: Arc<RwLock<ActiveState>>,
}

impl Callbacks {
    pub fn new(client: TonClient) -> Self {
        Self {
            client,
            state: Arc::new(RwLock::new(ActiveState::default())),
        }
    }

    pub fn take_messages(&self, common_queue: &mut VecDeque<String>) {
        let new_msgs = &mut self.state.write().unwrap().msg_queue;
        common_queue.append(new_msgs);
    }

    pub fn outputs(&self) -> Vec<String> {
        self.state.read().unwrap().outputs.clone()
    }
}

#[async_trait::async_trait]
impl BrowserCallbacks for Callbacks {
    /// Debot asks browser to print message to user
    fn log(&self, level: LogLevel, msg: String) {
        println!("[{:?}] {}", level, msg);
        if level == LogLevel::User {
            let mut state = self.state.write().unwrap();
            state.outputs.push(msg);
        }
    }

    /// Debot is switched to another context.
    async fn switch(&self, ctx_id: u8) {
        let mut state = self.state.write().unwrap();
        state.state_id = ctx_id;
        if ctx_id == STATE_EXIT {
            return;
        }

        state.active_actions = vec![];
    }

    async fn switch_completed(&self) {}

    /// Debot asks browser to show user an action from the context
    async fn show_action(&self, act: DAction) {
        let mut state = self.state.write().unwrap();
        println!("{}) {}", state.active_actions.len() + 1, act.desc);
        state.active_actions.push(act);
    }

    // Debot engine asks user to enter argument for an action.
    async fn input(&self, prefix: &str, value: &mut String) {
        let stdio = io::stdin();
        let mut reader = stdio.lock();
        let mut writer = io::stdout();
        *value = input(prefix, &mut reader, &mut writer);
    }

    /// Debot engine requests keys to sign something
    async fn get_signing_box(&self) -> Result<SigningBoxHandle, String> {
        panic!("unimplemented get_signing_box");
        //let handle = TerminalSigningBox::new::<&[u8]>(self.client.clone(), vec![], None)
        //    .await?
        //    .leak()
        //    .0;
        //Ok(SigningBoxHandle(handle))
    }

    /// Debot asks to run action of another debot
    async fn invoke_debot(&self, _debot: String, _action: DAction) -> Result<(), String> {
        Ok(())
    }

    async fn send(&self, message: String) {
        let mut state = self.state.write().unwrap();
        state.msg_queue.push_back(message);
    }

    async fn approve(&self, activity: DebotActivity) -> ClientResult<bool> {
        let approved = true;
        let mut info = String::new();
        info += "--------------------\n";
        info += "[Permission Request]\n";
        info += "--------------------\n";
        let prompt = match activity {
            DebotActivity::Transaction {
                msg: _,
                dst,
                out,
                fee,
                setcode,
                signkey,
                signing_box_handle: _,
            } => {
                info += "DeBot is going to create an onchain transaction.\n";
                info += "Details:\n";
                info += &format!("  account: {}\n", dst);
                info += &format!("  Transaction fees: {fee} tokens\n");
                if !out.is_empty() {
                    info += "  Outgoing transfers from the account:\n";
                    for spending in out {
                        info += &format!(
                            "    recipient: {}, amount: {} tokens\n",
                            spending.dst, spending.amount,
                        );
                    }
                } else {
                    info += "  No outgoing transfers from the account.\n";
                }
                info += &format!("  Message signer public key: {}\n", signkey);
                if setcode {
                    info += "  Warning: the transaction will change the account's code\n";
                }
                "Confirm the transaction (y/n)?"
            }
        };
        print!("{}", info);
        Ok(approved)
    }

    async fn fetch(
        &self,
        url: String,
        method: String,
        headers: Vec<FetchHeader>,
        body: Option<String>,
    ) -> ClientResult<FetchResponse> {
        Ok(FetchResponse::default())
    }

    async fn encrypt(&self, _handle: EncryptionBoxHandle, data: String) -> ClientResult<String> {
        Ok(data)
    }
    async fn decrypt(&self, _handle: EncryptionBoxHandle, data: String) -> ClientResult<String> {
        Ok(data)
    }
    /// Data signing
    /// data - string with data to sign encoded as base64.
    async fn sign(&self, _handle: SigningBoxHandle, data: String) -> ClientResult<String> {
        Ok(data)
    }
    async fn send_message(&self, message: String) -> ClientResult<ton_client::processing::ResultOfSendMessage> {
        ton_client::processing::send_message(
            self.client.clone(),
            ParamsOfSendMessage {
                message,
                ..Default::default()
            },
            |_| async {},
        )
        .await
    }
    async fn query(&self, params: ParamsOfQuery) -> ClientResult<ResultOfQuery> {
        ton_client::net::query(self.client.clone(), params).await
    }
    async fn query_collection(
        &self,
        params: ParamsOfQueryCollection,
    ) -> ClientResult<ResultOfQueryCollection> {
        ton_client::net::query_collection(self.client.clone(), params).await
    }
    async fn wait_for_collection(
        &self,
        params: ParamsOfWaitForCollection,
    ) -> ClientResult<ResultOfWaitForCollection> {
        ton_client::net::wait_for_collection(self.client.clone(), params).await
    }
    async fn wait_for_transaction(
        &self,
        params: WaitForTransactionParams,
    ) -> ClientResult<ResultOfProcessMessage> {
        ton_client::processing::wait_for_transaction(
            self.client.clone(),
            ParamsOfWaitForTransaction {
                abi: params.abi,
                message: params.message,
                shard_block_id: params.shard_block_id,
                send_events: params.send_events,
                sending_endpoints: params.sending_endpoints,
            },
            |_| async {},
        )
        .await
    }
    async fn query_transaction_tree(
        &self,
        params: ParamsOfQueryTransactionTree,
    ) -> ClientResult<ResultOfQueryTransactionTree> {
        ton_client::net::query_transaction_tree(self.client.clone(), params).await
    }
    async fn get_signing_box_info(&self, handle: SigningBoxHandle) -> ClientResult<String> {
        let res = ton_client::crypto::signing_box_get_public_key(
            self.client.clone(),
            RegisteredSigningBox { handle },
        )
        .await?;
        Ok(res.pubkey)
    }
    async fn get_encryption_box_info(
        &self,
        handle: EncryptionBoxHandle,
    ) -> ClientResult<EncryptionBoxInfo> {
        let res = ton_client::crypto::encryption_box_get_info(
            self.client.clone(),
            ParamsOfEncryptionBoxGetInfo {
                encryption_box: handle,
            },
        )
        .await?;
        Ok(res.info)
    }
}
