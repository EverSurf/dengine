use ton_client::boc::internal::serialize_object_to_base64;
use ton_client::encoding::account_decode;
use ton_client::error::ClientResult;
use ton_block::{InternalMessageHeader, Message};
use ton_types::SliceData;

pub(super) fn build_internal_message(
    src: &String,
    dst: &String,
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
fn now_ms() -> u64 {
    chrono::prelude::Utc::now().timestamp_millis() as u64
}
