use super::super::dinterface::{decode_answer_id, decode_bool_arg, decode_prompt, decode_string_arg};
use super::super::term_browser::terminal_input;
use serde_json::{Value, json};
use ton_client::abi::Abi;
use dengine::prelude::{DebotInterface, InterfaceResult, BrowserRef, LogLevel};
use ton_client::encoding::decode_abi_bigint;
use std::io::Read;

pub(super) const ID: &str = "8796536366ee21852db56dccb60bc564598b618c865fc50c8b1ab740bba128e3";

const ABI: &str = r#"
{
	"ABI version": 2,
	"version": "2.2",
	"header": ["time"],
	"functions": [
		{
			"name": "input",
            "id": "0x3955f72f",
			"inputs": [
				{"name":"answerId","type":"uint32"},
				{"name":"prompt","type":"string"},
				{"name":"multiline","type":"bool"}
			],
			"outputs": [
				{"name":"value","type":"string"}
			]
		},
		{
			"name": "print",
            "id": "0x0ce649c2",
			"inputs": [
				{"name":"answerId","type":"uint32"},
				{"name":"message","type":"string"}
			],
			"outputs": [
			]
		},
		{
			"name": "printf",
            "id": "0x36a926ce",
			"inputs": [
				{"name":"answerId","type":"uint32"},
				{"name":"fmt","type":"string"},
				{"name":"fargs","type":"cell"}
			],
			"outputs": [
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

pub struct Terminal {
    printer: BrowserRef,
}

impl Terminal {
    pub fn new(printer: BrowserRef) -> Self {
        Self {printer}
    }
    fn input_str(&self, args: &Value) -> InterfaceResult {
        let answer_id = decode_answer_id(args)?;
        let prompt = decode_prompt(args)?;
        let multiline = decode_bool_arg(args, "multiline")?;
        let mut value = String::new();
        if multiline {
            println!("{}", &prompt);
            if cfg!(windows) {
                println!("(Ctrl+Z to exit)");
            } else {
                println!("(Ctrl+D to exit)");
            }
            std::io::stdin().read_to_string(&mut value)
                .map_err(|e| format!("input error: {}", e))?;
            println!();
        } else {
            value = terminal_input(&prompt, |_val| Ok(()));
        }
        Ok((answer_id, json!({ "value": value })))
    }

    pub async fn print(&self, args: &Value) -> InterfaceResult {
        let answer_id = decode_answer_id(args)?;
        let message = decode_string_arg(args, "message")?;
		self.printer.log(LogLevel::User, message);
		Ok((answer_id, json!({})))
    }
}

#[async_trait::async_trait]
impl DebotInterface for Terminal {
    fn get_id(&self) -> String {
        ID.to_string()
    }

    fn get_abi(&self) -> Abi {
        Abi::Json(ABI.to_owned())
    }

    async fn call(&self, func: &str, args: &Value) -> InterfaceResult {
        match func {
            "input" => self.input_str(args),
            "print" => self.print(args).await,
            _ => Err(format!("function \"{}\" is not implemented", func)),
        }
    }
}