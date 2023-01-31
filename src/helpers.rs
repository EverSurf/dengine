use crate::sdk_prelude::{
    account_decode, serialize_object_to_base64, ClientResult, InternalMessageHeader, Message,
    SliceData,
};

pub(crate) fn build_internal_message(
    src: &str,
    dst: &str,
    body: SliceData,
) -> ClientResult<String> {
    let src_addr = account_decode(src)?;
    let dst_addr = account_decode(dst)?;
    let mut msg = Message::with_int_header(InternalMessageHeader::with_addresses(
        src_addr,
        dst_addr,
        Default::default(),
    ));
    msg.set_body(body);
    serialize_object_to_base64(&msg, "message")
}
pub(crate) fn now_ms() -> u64 {
    chrono::prelude::Utc::now().timestamp_millis() as u64
}
