use crate::action::DAction;
use serde::{de, Deserialize, Deserializer};
use std::fmt::Display;
use std::str::FromStr;
use ton_client::encoding::decode_abi_number;

pub const STATE_ZERO: u8 = 0;
pub const STATE_CURRENT: u8 = 253;
pub const STATE_PREV: u8 = 254;
pub const STATE_EXIT: u8 = 255;

/// Debot Context. Consists of several actions.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
#[derive(Clone)]
pub struct DContext {
    #[serde(deserialize_with = "from_abi_num")]
    pub id: u8,
    #[serde(deserialize_with = "from_hex_to_utf8_str")]
    pub desc: String,
    pub actions: Vec<DAction>,
}

impl DContext {
    #[allow(dead_code)]
    pub fn new(desc: String, actions: Vec<DAction>, id: u8) -> Self {
        DContext { desc, actions, id }
    }
}

pub(super) fn from_abi_num<'de, D>(des: D) -> Result<u8, D::Error>
where
    D: Deserializer<'de>,
{
    let s: String = Deserialize::deserialize(des)?;
    decode_abi_number(&s).map_err(de::Error::custom)
}

pub(super) fn str_hex_to_utf8(s: &str) -> Option<String> {
    String::from_utf8(hex::decode(s).ok()?).ok()
}

pub(super) fn from_hex_to_utf8_str<'de, S, D>(des: D) -> Result<S, D::Error>
where
    S: FromStr,
    S::Err: Display,
    D: Deserializer<'de>,
{
    let s: String = Deserialize::deserialize(des)?;
    let s = str_hex_to_utf8(&s)
        .ok_or_else(|| "failed to convert bytes to utf8 string".to_string())
        .unwrap();
    S::from_str(&s).map_err(de::Error::custom)
}
