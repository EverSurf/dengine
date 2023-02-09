//use super::term_signing_box::TerminalSigningBox;
use super::term_browser::{action_input, input, terminal_input};
use super::config::Config;
use super::helpers::TonClient;
use std::collections::VecDeque;
use std::io;
use std::sync::{Arc, RwLock};
use ton_client::crypto::SigningBoxHandle;
use dengine::prelude::*;
use ton_client::error::ClientResult;

#[derive(Default)]
struct ActiveState {
    state_id: u8,
    active_actions: Vec<DAction>,
    msg_queue: VecDeque<String>,
}

pub(super) struct Callbacks {
    client: TonClient,
    state: Arc<RwLock<ActiveState>>,
}

impl Callbacks {
    pub fn new(
        client: TonClient,
    ) -> Self {
        Self {
            client,
            state: Arc::new(RwLock::new(ActiveState::default())),
        }
    }

    pub fn select_action(&self) -> Option<DAction> {
        let state = self.state.read().unwrap();
        if state.state_id == STATE_EXIT {
            return None;
        }
        if state.active_actions.is_empty() {
            log::debug!("no more actions, exit loop");
            return None;
        }

        loop {
            let res = action_input(state.active_actions.len());
            if res.is_err() {
                println!("{}", res.unwrap_err());
                continue;
            }
            let (n, _, _) = res.unwrap();
            let act = state.active_actions.get(n - 1);
            if act.is_none() {
                println!("Invalid action. Try again.");
                continue;
            }
            return act.cloned();
        }
    }

    pub fn take_messages(&self, common_queue: &mut VecDeque<String>) {
        let new_msgs = &mut self.state.write().unwrap().msg_queue;
        common_queue.append(new_msgs);
    }
}

#[async_trait::async_trait]
impl BrowserCallbacks for Callbacks {
    /// Debot asks browser to print message to user
    async fn log(&self, msg: String) {
        println!("{}", msg);
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
        let mut approved = true;
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
                            spending.dst,
                            spending.amount,
                        );
                    }
                } else {
                    info += "  No outgoing transfers from the account.\n";
                }
                info += &format!("  Message signer public key: {}\n", signkey);
                if setcode {
                    info +=
                        "  Warning: the transaction will change the account's code\n";
                }
                "Confirm the transaction (y/n)?"
            }
        };
        print!("{}", info);
        Ok(approved)
    }
}
