use super::config::Config;
use super::helpers::{create_client, load_ton_address, load_abi, TonClient};
use std::io::{self, BufRead, Write};
use std::sync::Arc;
use serde_json::json;
use dengine::prelude::*;
use std::collections::{HashMap, VecDeque};
use super::callbacks::Callbacks;
use super::dinterface::SupportedInterfaces;
use ton_client::boc::{ParamsOfParse, parse_message};
use ton_client::crypto::{KeyPair};
use ton_client::abi::{Abi, CallSet, encode_internal_message, ParamsOfEncodeInternalMessage};

const BROWSER_ID: &str = "0000000000000000000000000000000000000000000000000000000000000000";
/// Stores Debot info needed for DBrowser.
struct DebotEntry {
    abi: Abi,
    dengine: DEngine,
    callbacks: Arc<Callbacks>,
    info: DebotInfo,
}

/// Top level object. Created only once.
struct TerminalBrowser {
    client: TonClient,
    /// common message queue for both inteface calls and invoke calls (from different debots).
    msg_queue: VecDeque<String>,
    /// Map of instantiated Debots. [addr] -> entry.
    /// New debots are created by invoke requests.
    bots: HashMap<String, DebotEntry>,
    /// Set of intrefaces implemented by current DBrowser.
    interfaces: SupportedInterfaces,
    config: Arc<Config>,
    /// Indicates if Browser will interact with the user or not.
    interactive: bool,
    /// Browser exit argument. Initialized only if DeBot sends message to the DeBot Browser address.
    pub exit_arg: Option<serde_json::Value>,
    callbacks: Arc<Callbacks>,
}

impl TerminalBrowser {
    async fn new(client: TonClient, addr: &str, config: Arc<Config>, debot_key: KeyPair) -> Result<Self, String> {
        let callbacks = Arc::new(Callbacks::new(client.clone()));
        let mut browser = Self {
            client: client.clone(),
            msg_queue: Default::default(),
            bots: HashMap::new(),
            interfaces: SupportedInterfaces::new(client, debot_key, callbacks.clone()),
            callbacks,
            config,
            interactive: false,
            exit_arg: None,
        };
        browser.fetch_debot(addr, true, true).await?;
        Ok(browser)
    }

    async fn fetch_debot(&mut self, addr: &str, call_start: bool, autorun: bool) -> Result<String, String> {
        let debot_addr = load_ton_address(addr, &self.config)?;
        let callbacks_ref = Arc::clone(&self.callbacks);
        let mut dengine = DEngine::new_with_client(
            debot_addr.clone(),
            None,
            self.client.clone(),
            self.callbacks.clone(),
        );
        let info: DebotInfo = dengine.init().await?.into();
        let abi_version = info.dabi_version.clone();
        let abi: Abi = Abi::Contract(serde_json::from_str::<AbiContract>(info.dabi.as_ref().unwrap()).unwrap());
        if !autorun {
            Self::print_info(&info);
        }
        let mut run_debot = autorun;
        if !run_debot {
            let _ = terminal_input("Run the DeBot (y/n)?", |val| {
                run_debot = match val.as_str() {
                    "y" => true,
                    "n" => false,
                    _ => return Err("invalid enter".to_string()),
                };
                Ok(())
            });
        }
        if !run_debot {
            return Err("DeBot rejected".to_string());
        }
        if call_start {
            dengine.start().await?;
        }
        callbacks_ref.take_messages(&mut self.msg_queue);

        self.bots.insert(
            debot_addr,
            DebotEntry {
                abi,
                dengine,
                callbacks: callbacks_ref,
                info,
            }
        );
        Ok(abi_version)
    }

    async fn call_interface(
        &mut self,
        msg: String,
        interface_id: String,
        debot_addr: &str,
    ) -> Result<(), String> {
        let debot = self.bots.get_mut(debot_addr)
            .ok_or_else(|| "Internal browser error: debot not found".to_owned())?;
        if let Some(result) = self.interfaces.try_execute(&msg, &interface_id, &debot.info.dabi_version).await {
            let (func_id, return_args) = result?;
            log::debug!("response: {} ({})", func_id, return_args);
            let call_set = match func_id {
                0 => None,
                _ => CallSet::some_with_function_and_input(&format!("0x{:x}", func_id), return_args),
            };
            let response_msg = encode_internal_message(
                self.client.clone(),
                ParamsOfEncodeInternalMessage {
                    abi: Some(debot.abi.clone()),
                    address: Some(debot_addr.to_owned()),
                    call_set,
                    value: "1000000000000000".to_owned(),
                    ..Default::default()
                }
            )
            .map_err(|e| format!("{}", e))?
            .message;
            let result = debot.dengine.send(response_msg).await;
            debot.callbacks.take_messages(&mut self.msg_queue);
            if let Err(e) = result {
                println!("Debot error: {}", e);
            }
        }

        Ok(())
    }

    async fn call_debot(&mut self, addr: &str, msg: String) -> Result<(), String> {
        if self.bots.get_mut(addr).is_none() {
            self.fetch_debot(addr, false, !self.interactive).await?;
        }
        let debot = self.bots.get_mut(addr).ok_or("Internal error: debot not found")?;
        debot.dengine.send(msg).await.map_err(|e| format!("Debot failed: {}", e))?;
        debot.callbacks.take_messages(&mut self.msg_queue);
        Ok(())
    }

    fn print_info(info: &DebotInfo) {
        println!("DeBot Info:");
        fn print(field: &Option<String>) -> &str {
            field.as_ref().map(|v| v.as_str()).unwrap_or("None")
        }
        println!("Name   : {}", print(&info.name));
        println!("Version: {}", print(&info.version));
        println!("Author : {}", print(&info.author));
        println!("Publisher: {}", print(&info.publisher));
        println!("Support: {}", print(&info.support));
        println!("Description: {}", print(&info.caption));
        println!("{}", print(&info.hello));
    }

    async fn set_exit_arg(&mut self, _message: String, _debot_addr: &str) -> Result<(), String> {
        //let abi = Abi;
        //let arg = if let Some(abi) = abi {
        //    let decoded = decode_message(
        //        self.client.clone(),
        //        ParamsOfDecodeMessage { 
        //            abi,
        //            message,
        //            ..Default::default()
        //         },
        //    ).await.map_err(|e| format!("{}", e))?;
        //    decoded.value.unwrap_or(json!({}))
        //} else {
        //    json!({"message": message})
        //};
        //self.exit_arg = Some(arg);
        Ok(())
    }

    pub fn exit(self) -> (Option<serde_json::Value>, Vec<String>) {
        (self.exit_arg, self.callbacks.outputs())
    }

}

pub(crate) fn input<R, W>(prefix: &str, reader: &mut R, writer: &mut W) -> String
where
    R: BufRead,
    W: Write,
{
    let mut input_str = "".to_owned();
    let mut argc = 0;
    while argc == 0 {
        println!("{}", prefix);
        if let Err(e) = writer.flush() {
            println!("failed to flush: {}", e);
            return input_str;
        }
        if let Err(e) = reader.read_line(&mut input_str) {
            println!("failed to read line: {}", e);
            return input_str;
        }
        argc = input_str
            .split_whitespace()
            .count();
    }
    input_str.trim().to_owned()
}

pub(crate) fn terminal_input<F>(prompt: &str, mut validator: F) -> String
where
    F: FnMut(&String) -> Result<(), String>
{
    let stdio = io::stdin();
    let mut reader = stdio.lock();
    let mut writer = io::stdout();
    let mut value = input(prompt, &mut reader, &mut writer);
    while let Err(e) = validator(&value) {
        println!("{}. Try again.", e);
        value = input(prompt, &mut reader, &mut writer);
    }
    value
}

/// Starts Terminal DeBot Browser with main DeBot.
///
/// Fetches DeBot by address from blockchain and runs it according to pipechain.
pub async fn run_debot_browser(
    ton: TonClient,
    debot_addr: &str,
    config: Arc<Config>,
    debot_key: KeyPair,
) -> Result<(Option<serde_json::Value>, Vec<String>), String> {

    let mut browser = TerminalBrowser::new(ton.clone(), debot_addr, config, debot_key).await?;
        let mut next_msg = browser.msg_queue.pop_front();
        while let Some(msg) = next_msg {
            let parsed = parse_message(
                ton.clone(),
                ParamsOfParse { boc: msg.clone(), ..Default::default() },
            )
            .map_err(|e| format!("{}", e))?
            .parsed;

            let msg_dest = parsed["dst"].as_str()
                .ok_or("invalid message in the queue: no dst address".to_string())?;

            let msg_src = parsed["src"].as_str()
                .ok_or("invalid message in the queue: no src address".to_string())?;

            let wc_and_addr: Vec<_> = msg_dest.split(':').collect();
            let id = wc_and_addr[1].to_string();
            let wc = i8::from_str_radix(wc_and_addr[0], 10).map_err(|e| format!("{}", e))?;

            if wc == DEBOT_WC {
                if id == BROWSER_ID {
                    // Message from DeBot to Browser
                    browser.set_exit_arg(msg, msg_src).await?;
                } else {
                    browser.call_interface(msg, id, msg_src).await?;
                }
            } else {
                browser.call_debot(msg_dest, msg).await?;
            }

            next_msg = browser.msg_queue.pop_front();
        }

    Ok(browser.exit())
}

#[cfg(test)]
mod tests {}
