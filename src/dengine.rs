use crate::action::{AcType, DAction};
use crate::common::*;
use crate::context::{DContext, STATE_CURRENT, STATE_EXIT, STATE_PREV, STATE_ZERO};
use crate::builtin_interfaces::*;
use crate::routines;
use crate::sdk_prelude::*;
use std::collections::VecDeque;
use ton_abi::Contract;
use std::future::Future;

const EMPTY_CELL: &str = "te6ccgEBAQEAAgAAAA==";

fn create_client(endpoints: Option<Vec<String>>) -> Result<TonClient, String> {
    let cli_conf = ClientConfig {
        network: NetworkConfig {
            endpoints,
            ..Default::default()
        },
        ..Default::default()
    };
    let cli =
        ClientContext::new(cli_conf).map_err(|e| format!("failed to create tonclient: {e}"))?;
    Ok(Arc::new(cli))
}

fn load_abi(abi: &str) -> Result<Abi, String> {
    Ok(Abi::Contract(
        serde_json::from_str(abi).map_err(|e| format!("failed to parse abi: {e}"))?,
    ))
}

// TODO: implement address validation
pub fn load_ton_address(addr: &str) -> Result<String, String> {
    Ok(addr.to_owned())
}

//const OPTION_ABI: u8 = 1;
const OPTION_TARGET_ABI: u8 = 2;
const OPTION_TARGET_ADDR: u8 = 4;

/// Debot Engine.
/// 
/// Downloads and stores debot, executes its functions and calls
/// Debot Browser callbacks.
pub struct DEngine {
    raw_abi: String,
    abi: Abi,
    addr: String,
    ton: TonClient,
    state: String,
    state_machine: Vec<DContext>,
    curr_state: u8,
    prev_state: u8,
    target_addr: Option<String>,
    target_abi: Option<String>,
    browser: BrowserRef,
    builtin_interfaces: BuiltinInterfaces,
    info: DInfo,
}

impl DEngine {
    pub fn new(
        addr: String,
        abi: Option<String>,
        endpoints: Option<Vec<String>>,
        browser: BrowserRef,
    ) -> Self {
        DEngine::new_with_client(addr, abi, create_client(endpoints).unwrap(), browser)
    }

    pub fn new_with_client(
        addr: String,
        abi: Option<String>,
        ton: TonClient,
        browser: BrowserRef,
    ) -> Self {
        let abi = abi
            .map_or_else(|| load_abi(DEBOT_ABI), |s| load_abi(&s))
            .unwrap();
        DEngine {
            raw_abi: String::new(),
            abi,
            addr,
            ton: ton.clone(),
            state: String::new(),
            state_machine: vec![],
            curr_state: STATE_EXIT,
            prev_state: STATE_ZERO,
            target_addr: None,
            target_abi: None,
            browser: browser.clone(),
            builtin_interfaces: BuiltinInterfaces::new(ton, browser),
            info: Default::default(),
        }
    }

    pub async fn fetch(client: TonClient, addr: String) -> Result<DInfo, String> {
        let state = Self::fetch_state_with_client(client.clone(), addr.clone()).await?;
        Self::fetch_info_from_state(client, addr, state).await
    }

    pub async fn init(&mut self) -> Result<DInfo, String> {
        self.fetch_state_and_info().await?;
        self.prev_state = STATE_EXIT;
        Ok(self.info.clone())
    }

    pub async fn start(&mut self) -> Result<(), String> {
        self.switch_state(STATE_ZERO, true).await?;
        Ok(())
    }

    async fn fetch_info_from_state(ton: TonClient, addr: String, state: String) -> Result<DInfo, String> {
        let dabi_version = fetch_target_abi_version(ton.clone(), state.clone())
            .await
            .map_err(|e| e.to_string())?;
        let abi: Abi = load_abi(DEBOT_ABI).unwrap();
        let result = Self::run(
            ton.clone(),
            state.clone(),
            addr.clone(),
            abi.clone(),
            "getRequiredInterfaces",
            None,
        )
        .await;
        let interfaces: Vec<String> = match result {
            Ok(r) => {
                let mut output = r.return_value.unwrap_or(json!({}));
                serde_json::from_value(output["interfaces"].take()).map_err(|e| {
                    format!(
                    "failed to parse \"interfaces\" returned from \"getRequiredInterfaces\": {e}"
                )
                })?
            }
            Err(_) => vec![],
        };

        let result = Self::run(
            ton.clone(),
            state.clone(),
            addr.clone(),
            abi.clone(),
            "getDebotInfo",
            None,
        )
        .await;
        let mut info: DInfo = match result {
            Ok(r) => parse_debot_info(r.return_value)?,
            Err(_) => Default::default(),
        };

        info.interfaces = interfaces;
        info.dabi_version = dabi_version;
        Ok(info)
    }

    async fn fetch_state_and_info(&mut self) -> Result<(), String> {
        self.state = self.fetch_state(self.addr.clone()).await?;
        self.info =
            Self::fetch_info_from_state(self.ton.clone(), self.addr.clone(), self.state.clone()).await?;
        if let Some(dabi) = self.info.dabi.as_ref() {
            self.raw_abi = dabi.clone();
            self.abi = load_abi(&self.raw_abi)?;
            self.builtin_interfaces.add(Arc::new(MsgInterface::new(
                self.ton.clone(),
                self.addr.clone(),
                self.abi.clone(),
                self.browser.clone(),
            )));
            self.builtin_interfaces
                .add(Arc::new(JsonInterface::new(&self.raw_abi)));
        }
        //self.update_options().await?;
        let mut context_vec = vec![];
        let mut start_act = DAction::new(
            String::new(),
            "start".to_owned(),
            AcType::RunAction as u8,
            STATE_CURRENT,
        );
        start_act.attrs = "instant".to_owned();
        start_act.misc = EMPTY_CELL.to_owned();
        context_vec.push(DContext::new(String::new(), vec![start_act], STATE_ZERO));
        self.state_machine = context_vec;
        Ok(())
    }

    pub async fn send(&mut self, message: String) -> ClientResult<()> {
        let output = self.send_to_debot(message).await?;
        self.handle_output(output).await
    }

    async fn run_debot_internal(
        &mut self,
        source: String,
        func_id: u32,
        params: JsonValue,
    ) -> ClientResult<RunOutput> {
        debug!(self.browser, "send from {} id = {} params = {}", source, func_id, params);
        let abi = Contract::load(self.raw_abi.as_bytes())
            .map_err(|e| Error::invalid_debot_abi(e.to_string()))?;
        let func_name = &abi
            .function_by_id(func_id, true)
            .map_err(Error::invalid_function_id)?
            .name;

        let msg_params = ParamsOfEncodeMessageBody {
            abi: self.abi.clone(),
            signer: Signer::None,
            processing_try_index: None,
            is_internal: true,
            call_set: CallSet::some_with_function_and_input(func_name, params).unwrap(),
            address: Some(self.addr.clone()),
            ..Default::default()
        };
        let body = encode_message_body(self.ton.clone(), msg_params)
            .await?
            .body;
        let (_, body_cell) = deserialize_cell_from_base64(&body, "message body")?;
        let body = slice_from_cell(body_cell)?;
        let msg_base64 = build_internal_message(&source, &self.addr, body)?;
        self.send_to_debot(msg_base64).await
    }

    async fn send_to_debot(&mut self, msg: String) -> ClientResult<RunOutput> {
        let run_result = run_tvm(
            self.ton.clone(),
            ParamsOfRunTvm {
                account: self.state.clone(),
                message: msg,
                abi: Some(self.abi.clone()),
                return_updated_account: Some(true),
                ..Default::default()
            },
        )
        .await?;
        let mut run_output = RunOutput::new(
            run_result.account,
            self.addr.clone(),
            run_result.decoded.and_then(|x| x.output),
            run_result.out_messages,
        )?;
        self.state = std::mem::take(&mut run_output.account);
        Ok(run_output)
    }

    async fn handle_action(&mut self, a: &DAction) -> Result<Option<Vec<DAction>>, String> {
        match a.action_type {
            AcType::Empty => {
                debug!(self.browser, "empty action: {}", a.name);
                Ok(None)
            }
            AcType::RunAction => {
                debug!(self.browser, "run_action: {}", a.name);
                let result = self.run_action(a).await?;
                let actions = result.decode_actions();
                self.handle_output(result)
                    .await
                    .map_err(|e| e.to_string())?;
                actions
            }
            AcType::RunMethod => {
                debug!(self.browser, "run_getmethod: {}", a.func_attr().unwrap());
                let args: Option<JsonValue> = if let Some(getter) = a.args_attr() {
                    self.run_debot_external(&getter, None).await?.return_value
                } else {
                    None
                };
                self.run_getmethod(&a.func_attr().unwrap(), args, &a.name)
                    .await?;
                Ok(None)
            }
            AcType::SendMsg => {
                debug!(self.browser, "sendmsg: {}", a.name);
                let signer = if a.sign_by_user() {
                    Some(self.browser.get_signing_box().await?)
                } else {
                    None
                };
                let args: Option<JsonValue> = if a.misc != EMPTY_CELL {
                    Some(json!({ "misc": a.misc }))
                } else {
                    None
                };
                let result = self.run_sendmsg(&a.name, args, signer.clone()).await?;
                if let Some(signing_box) = signer {
                    let _ = remove_signing_box(
                        self.ton.clone(),
                        RegisteredSigningBox {
                            handle: signing_box,
                        },
                    );
                }
                self.browser.log(LogLevel::User, "Transaction succeeded.".to_string());
                result.map(|r| self.browser.log(LogLevel::User, format!("Result: {r}")));
                Ok(None)
            }
            AcType::Invoke => {
                debug!(self.browser, "invoke debot: run {}", a.name);
                let result = self.run_debot_external(&a.name, None).await?.return_value;
                let invoke_args = result.ok_or(format!(
                    r#"invalid invoke action "{}": it must return "debot" and "action" arguments"#,
                    a.name
                ))?;
                debug!(self.browser, "{}", invoke_args);
                let debot_addr = load_ton_address(invoke_args["debot"].as_str().unwrap())?;
                let debot_action: DAction =
                    serde_json::from_value(invoke_args["action"].clone()).unwrap();
                debug!(self.browser, 
                    "invoke debot: {}, action name: {}",
                    &debot_addr, debot_action.name
                );
                self.browser.invoke_debot(debot_addr, debot_action).await?;
                debug!(self.browser, "invoke completed");
                Ok(None)
            }
            AcType::Print => {
                debug!(self.browser, "print action: {}", a.name);
                let label = if let Some(args_getter) = a.format_args() {
                    let args = if a.misc != EMPTY_CELL {
                        Some(json!({"misc": a.misc}))
                    } else {
                        None
                    };
                    self.run_debot_external(&args_getter, args)
                        .await?
                        .return_value
                        .map(|p| routines::format_string(&a.name, &p))
                        .unwrap_or_default()
                } else {
                    a.name.clone()
                };
                self.browser.log(LogLevel::User, label);
                Ok(None)
            }
            AcType::Goto => {
                debug!(self.browser, "goto action");
                Ok(None)
            }
            AcType::CallEngine => {
                panic!();
            }
            _ => {
                let err_msg = "unsupported action type".to_owned();
                self.browser.log(LogLevel::Error, err_msg.clone());
                Err(err_msg)
            }
        }
    }

    async fn switch_state(&mut self, mut state_to: u8, force: bool) -> Result<(), String> {
        debug!(self.browser, "switching to {}", state_to);
        if state_to == STATE_CURRENT {
            state_to = self.curr_state;
        }
        if state_to == STATE_PREV {
            state_to = self.prev_state;
        }
        if state_to == STATE_EXIT {
            self.browser.switch(STATE_EXIT).await;
            self.browser.switch_completed().await;
        } else if state_to != self.curr_state || force {
            let mut instant_switch = true;
            self.prev_state = self.curr_state;
            self.curr_state = state_to;
            while instant_switch {
                // TODO: restrict cyclic switches
                let jump_to_ctx = self
                    .state_machine
                    .iter()
                    .find(|ctx| ctx.id == state_to)
                    .cloned();
                if let Some(ctx) = jump_to_ctx {
                    self.browser.switch(state_to).await;
                    instant_switch = self.enumerate_actions(ctx).await?;
                    state_to = self.curr_state;
                    self.browser.switch_completed().await;
                } else if state_to == STATE_EXIT {
                    self.browser.switch(STATE_EXIT).await;
                    self.browser.switch_completed().await;
                    instant_switch = false;
                } else {
                    self.browser
                        .log(LogLevel::Error, format!("Debot context #{state_to} not found. Exit."));
                    instant_switch = false;
                }
            }
        }
        Ok(())
    }

    async fn enumerate_actions(&mut self, ctx: DContext) -> Result<bool, String> {
        // find, execute and remove instant action from context.
        // if instant action returns new actions then execute them and insert into context.
        for action in &ctx.actions {
            let mut sub_actions = VecDeque::new();
            sub_actions.push_back(action.clone());
            while let Some(act) = sub_actions.pop_front() {
                if act.is_instant() {
                    if !act.desc.is_empty() {
                        self.browser.log(LogLevel::User, act.desc.clone());
                    }
                    if let Some(vec) = self.handle_action(&act).await? {
                        vec.iter().for_each(|a| sub_actions.push_back(a.clone()));
                    };
                    // if instant action wants to switch context then exit and do switch.
                    let to = if act.to == STATE_CURRENT {
                        self.curr_state
                    } else if act.to == STATE_PREV {
                        self.prev_state
                    } else {
                        act.to
                    };
                    if to != self.curr_state {
                        self.curr_state = act.to;
                        return Ok(true);
                    }
                } else if act.is_engine_call() {
                    self.handle_action(&act).await?;
                } else {
                    self.browser.show_action(act).await;
                }
            }
        }
        Ok(false)
    }

    async fn run_debot_external(
        &mut self,
        name: &str,
        args: Option<JsonValue>,
    ) -> Result<RunOutput, String> {
        let res = Self::run(
            self.ton.clone(),
            self.state.clone(),
            self.addr.clone(),
            self.abi.clone(),
            name,
            args,
        )
        .await;
        match res {
            Ok(res) => {
                self.state = res.account.clone();
                Ok(res)
            }
            Err(e) => {
                error!(self.browser, "{}", e);
                Err(self.handle_sdk_err(e).await)
            }
        }
    }

    async fn run_action(&mut self, action: &DAction) -> Result<RunOutput, String> {
        let args = self.query_action_args(action).await?;
        self.run_debot_external(&action.name, args).await
    }

    async fn run_sendmsg(
        &mut self,
        name: &str,
        args: Option<JsonValue>,
        signer: Option<SigningBoxHandle>,
    ) -> Result<Option<JsonValue>, String> {
        let result = self.run_debot_external(name, args).await?.return_value;
        if result.is_none() {
            return Err(format!(
                r#"action "{name}" is invalid: it must return "dest" and "body" arguments"#
            ));
        }
        let result = result.unwrap();
        let dest = result["dest"].as_str().unwrap();
        let body = result["body"].as_str().unwrap();
        let state = result["state"].as_str();

        let call_itself = load_ton_address(dest)? == self.addr;
        let abi = if call_itself {
            self.abi.clone()
        } else {
            load_abi(
                self.target_abi
                    .as_ref()
                    .ok_or_else(|| "target abi is undefined".to_string())?,
            )?
        };

        let res = decode_message_body(
            self.ton.clone(),
            ParamsOfDecodeMessageBody {
                abi: abi.clone(),
                body: body.to_string(),
                is_internal: true,
                ..Default::default()
            },
        )
        .map_err(|e| format!("failed to decode msg body: {e}"))?;

        debug!(self.browser, "calling {} at address {}", res.name, dest);
        debug!(self.browser, "args: {}", res.value.as_ref().unwrap_or(&json!({})));
        self.call_target(dest, abi, &res.name, res.value.clone(), signer, state)
            .await
    }

    async fn run_getmethod(
        &mut self,
        getmethod: &str,
        args: Option<JsonValue>,
        result_handler: &str,
    ) -> Result<Option<JsonValue>, String> {
        self.update_options().await?;
        if self.target_addr.is_none() {
            return Err("target address is undefined".to_string());
        }
        let (addr, abi) = self.get_target()?;
        let state = self.fetch_state(addr.clone()).await?;
        let result = Self::run(self.ton.clone(), state, addr, abi, getmethod, args).await;
        let result = match result {
            Ok(r) => Ok(r.return_value),
            Err(e) => Err(self.handle_sdk_err(e).await),
        }?;
        let result = self.run_debot_external(result_handler, result).await?;
        Ok(result.return_value)
    }

    pub(crate) async fn fetch_state(&self, addr: String) -> Result<String, String> {
        let b = self.browser.clone();
        let closure = |addr: String| {
            async move {
                b.query_collection(
                ParamsOfQueryCollection {
                        collection: "accounts".to_owned(),
                        filter: Some(serde_json::json!({
                            "id": { "eq": addr }
                        })),
                        result: "boc".to_owned(),
                        limit: Some(1),
                        order: None,
                    }
                ).await
            }
        };
        Self::load_state_by_query(closure, addr).await
    }

    pub(crate) async fn load_state_by_query<R: Future<Output = ClientResult<ResultOfQueryCollection>> + Send>(
        query: impl FnOnce(String) -> R,
        addr: String,
    ) -> Result<String, String> {
        let account_request = query(addr.clone()).await;
        let acc: ResultOfQueryCollection = account_request
            .map_err(|e| format!("failed to query account: {e}"))?;
        if acc.result.is_empty() {
            return Err(format!(
                "Cannot find smart contract with address {addr}"
            ));
        }
        let state = acc.result[0]["boc"].as_str().unwrap().to_owned();
        Ok(state)
    }

    pub(crate) async fn fetch_state_with_client(cli: TonClient, addr: String) -> Result<String, String> {
        let closure = |addr: String| {
            async move {
                ton_client::net::query_collection(
              cli,
               ParamsOfQueryCollection {
                        collection: "accounts".to_owned(),
                        filter: Some(serde_json::json!({
                            "id": { "eq": addr }
                        })),
                        result: "boc".to_owned(),
                        limit: Some(1),
                        order: None,
                    }
                ).await
            }
        };
        Self::load_state_by_query(closure, addr).await
    }

    async fn update_options(&mut self) -> Result<(), String> {
        let params = self
            .run_debot_external("getDebotOptions", None)
            .await?
            .return_value;
        let params = params.ok_or_else(|| "no return value".to_string())?;
        let opt_str = params["options"].as_str().unwrap();
        let options = decode_abi_number::<u8>(opt_str).unwrap();
        if options & OPTION_TARGET_ABI != 0 {
            self.target_abi = str_hex_to_utf8(params["targetAbi"].as_str().unwrap());
        }
        if (options & OPTION_TARGET_ADDR) != 0 {
            let addr = params["targetAddr"].as_str().unwrap();
            self.target_addr = Some(load_ton_address(addr)?);
        }
        Ok(())
    }

    async fn query_action_args(&self, act: &DAction) -> Result<Option<JsonValue>, String> {
        let args: Option<JsonValue> = if act.misc != EMPTY_CELL {
            Some(json!({ "misc": act.misc }))
        } else {
            if let Abi::Contract(ref abi_obj) = self.abi {
                let func = abi_obj.functions
                    .iter()
                    .find(|f| f.name == act.name)
                    .ok_or_else(|| "action not found".to_string())?;
                let mut args_json = json!({});
                for arg in &func.inputs {
                    let mut value = String::new();
                    self.browser.input("", &mut value).await;
                    if arg.param_type == "bytes" {
                        value = hex::encode(value.as_bytes());
                    }
                    args_json[&arg.name] = json!(&value);
                } 
                Some(args_json) 
            } else {
                unreachable!();
            }
        };
        Ok(args)
    }

    fn get_target(&self) -> Result<(String, Abi), String> {
        let addr = self
            .target_addr
            .clone()
            .ok_or_else(|| "target address is undefined".to_string())?;
        let abi = self
            .target_abi
            .as_ref()
            .ok_or_else(|| "target abi is undefined".to_string())?;
        let abi_obj = load_abi(abi)?;
        Ok((addr, abi_obj))
    }

    async fn run(
        ton: TonClient,
        state: String,
        addr: String,
        abi: Abi,
        func: &str,
        args: Option<JsonValue>,
    ) -> Result<RunOutput, ClientError> {
        let msg_params = ParamsOfEncodeMessage {
            abi: abi.clone(),
            address: Some(addr.clone()),
            deploy_set: None,
            call_set: if args.is_none() {
                CallSet::some_with_function(func)
            } else {
                CallSet::some_with_function_and_input(func, args.unwrap())
            },
            signer: Signer::None,
            processing_try_index: None,
            ..Default::default()
        };

        let result = encode_message(ton.clone(), msg_params).await?;

        let result = run_tvm(
            ton.clone(),
            ParamsOfRunTvm {
                account: state,
                message: result.message,
                abi: Some(abi),
                return_updated_account: Some(true),
                ..Default::default()
            },
        )
        .await;

        match result {
            Ok(res) => RunOutput::new(
                res.account,
                addr,
                res.decoded.and_then(|x| x.output),
                res.out_messages,
            ),
            Err(e) => Err(e),
        }
    }

    async fn call_target(
        &self,
        dest: &str,
        abi: Abi,
        func: &str,
        args: Option<JsonValue>,
        signer: Option<SigningBoxHandle>,
        state: Option<&str>,
    ) -> Result<Option<JsonValue>, String> {
        let addr = load_ton_address(dest)?;

        let call_params = ParamsOfEncodeMessage {
            abi: abi.clone(),
            address: Some(addr),
            deploy_set: state.and_then(|s| DeploySet::some_with_tvc(Some(s.to_string()))),
            call_set: if args.is_none() {
                CallSet::some_with_function(func)
            } else {
                CallSet::some_with_function_and_input(func, args.unwrap())
            },
            signer: match signer {
                Some(signing_box) => Signer::SigningBox {
                    handle: signing_box,
                },
                None => Signer::None,
            },
            processing_try_index: None,
            ..Default::default()
        };

        let browser = self.browser.clone();
        let callback = move |event| {
            let browser = browser.clone();
            debug!(browser, "{:?}", event);
            async move {
                if let ProcessingEvent::WillSend {
                    shard_block_id: _,
                    message_id,
                    message: _,
                    ..
                } = event
                {
                    browser.log(LogLevel::User, format!("Sending message {message_id}"));
                };
            }
        };

        match process_message(
            self.ton.clone(),
            ParamsOfProcessMessage {
                message_encode_params: call_params,
                send_events: true,
            },
            callback,
        )
        .await
        {
            Ok(res) => Ok(res.decoded.unwrap().output),
            Err(e) => {
                error!(self.browser, "{:?}", e);
                Err(self.handle_sdk_err(e).await)
            }
        }
    }

    async fn handle_output(&mut self, mut output: RunOutput) -> ClientResult<()> {
        while let Some(call) = output.pop() {
            match call {
                DebotCallType::Interface { msg, id } => {
                    debug!(self.browser, "Interface call");
                    match self
                        .builtin_interfaces
                        .try_execute(&msg, &id, &self.info.dabi_version)
                        .await
                    {
                        None => self.browser.send(msg).await,
                        Some(result) => {
                            let (fname, args) = result.map_err(Error::execute_failed)?;
                            let new_outputs = self
                                .run_debot_internal(format!("{DEBOT_WC}:{id}"), fname, args)
                                .await?;
                            output.append(new_outputs);
                        }
                    }
                }
                DebotCallType::GetMethod { msg, dest } => {
                    debug!(self.browser, "GetMethod call");
                    let target_state = self.fetch_state(dest.clone())
                        .await
                        .map_err(Error::execute_failed)?;
                    let callobj = ContractCall::new(
                        self.browser.clone(),
                        self.ton.clone(),
                        msg,
                        Signer::None,
                        target_state,
                        self.addr.clone(),
                        true,
                    )
                    .await?;
                    let answer_msg = callobj.execute(true).await?;
                    output.append(self.send_to_debot(answer_msg).await?);
                }
                DebotCallType::External { msg, dest } => {
                    debug!(self.browser, "External call");
                    let target_state = self.fetch_state(dest.clone())
                        .await
                        .map_err(Error::execute_failed)?;
                    let callobj = ContractCall::new(
                        self.browser.clone(),
                        self.ton.clone(),
                        msg,
                        Signer::None,
                        target_state,
                        self.addr.clone(),
                        false,
                    )
                    .await?;
                    let answer_msg = callobj.execute(true).await?;
                    output.append(self.send_to_debot(answer_msg).await?);
                }
                DebotCallType::Invoke { msg } => {
                    debug!(self.browser, "Invoke call");
                    self.browser.send(msg).await;
                }
            }
        }
        Ok(())
    }

    async fn handle_sdk_err(&self, err: ClientError) -> String {
        if err.code == SdkErrorCode::EncodeDeployMessageFailed as u32
            || err.code == SdkErrorCode::EncodeRunMessageFailed as u32
        {
            // when debot's function argument has invalid format
            "Invalid parameter".to_string()
        } else if err.code >= (ClientError::TVM as u32)
            && err.code < (ClientError::PROCESSING as u32)
        {
            // when debot function throws an exception
            if let Some(e) = err.data["exit_code"].as_i64() {
                Self::run(
                    self.ton.clone(),
                    self.state.clone(),
                    self.addr.clone(),
                    self.abi.clone(),
                    "getErrorDescription",
                    Some(json!({ "error": e })),
                )
                .await
                .ok()
                .and_then(|res| {
                    res.return_value.and_then(|v| {
                        v["desc"].as_str().and_then(|hex| {
                            hex::decode(hex)
                                .ok()
                                .and_then(|vec| String::from_utf8(vec).ok())
                        })
                    })
                })
                .unwrap_or(err.message)
            } else {
                err.message
            }
        } else {
            err.message
        }
    }
}
