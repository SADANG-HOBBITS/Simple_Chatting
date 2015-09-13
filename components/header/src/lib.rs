extern crate bincode;
extern crate rustc_serialize;

use bincode::rustc_serialize::{encode, decode};

pub const HEADER_SIZE: usize = 24;

pub const MESSAGE_TAG_ISUUE_ID: u64 = 0x0;
pub const MESSAGE_TAG_CHAT: u64 = 0x1;
pub const MESSAGE_TAG_CLOSE: u64 = 0x2;

/// 각각의 메시지에 필요한 정보 정의
/// 본문 길이, 메시지 종류, 전송한 클라이언트 식별자
#[derive(Debug, RustcEncodable, RustcDecodable)]
pub struct Header {
    pub length: usize,
    pub message_tag: u64,
    pub sender_id: u64,
}

impl Header {
	pub fn new(length: usize, message_tag: u64, sender_id: u64) -> Header {
		Header{ length: length, message_tag: message_tag, sender_id: sender_id }
	}
}

/// 헤더 객체를 바이트 Vector로 반환
/// parameter header: 인코드할 헤더 reference
/// return: byte Vector
pub fn encode_header(header: &Header) -> Vec<u8> {
	encode(header, bincode::SizeLimit::Infinite).unwrap()
}

/// byte Vector를 헤더 객체로 반환
/// parameter bytes: 디코드할 바이트 벡터 reference
/// return: Header
pub fn decode_header(bytes: &[u8]) -> Header {
	decode(bytes).unwrap()
}