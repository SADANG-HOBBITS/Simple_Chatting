extern crate header;extern crate bincode;
extern crate rustc_serialize;

use std::io::prelude::*;
use std::sync::mpsc::Sender;
use std::net::TcpStream;
use header::*;

/// stream을 통해서 Header와 String을 전송
/// parameter1 stream: 전송 TcpStream
/// parameter2 header: 전송할 Header 데이터 reference
/// parameter3 message: 전송할 String 데이터 reference (option)
pub fn send_message(mut stream: &TcpStream, header: &Header, message: Option<String>) {
    // 지정된 read buffer 사이즈보다 큰 메시지는 전송하지 않음
    if header.length > CLIENT_BUFFER_SIZE-HEADER_SIZE { return; }

    let header_bytes = encode_header(header);
    stream.write(&header_bytes[..]).unwrap();
    
    match message {
        Some(content) => {
            let message_bytes = content.into_bytes();
            stream.write(&message_bytes[..]).unwrap();
        },
        _ => {},
    }

    stream.flush();
}

pub const CLIENT_BUFFER_SIZE: usize = 512;
const CLOSE_CODE: usize = !0;

/// event handler에서 처리하는 이벤트
pub enum Signal {
    NewClient(Client),
    NewMessage(Header, String),
    Close(u64),
}

/// 바이트 스트림 데이터를 메시지로 조합한 결과
pub enum MessageStatus {
    Message(Header, String),
    NoMessage,
    Broken,
}

/// 전송받은 데이터를 읽어서 메시지로 조합
pub struct Client {
    pub id: u64,
    pub stream: TcpStream,
    pub recv_buf : [u8; CLIENT_BUFFER_SIZE],
    pub read_buf : [u8; CLIENT_BUFFER_SIZE],
    pub end_idx: usize,
    pub tx: Option<Sender<Signal>>,
}

impl Client {
    pub fn new(stream: TcpStream, tx: Option<Sender<Signal>>) -> Client {
        Client{ id:0, stream: stream, recv_buf: [0;CLIENT_BUFFER_SIZE], read_buf: [0;CLIENT_BUFFER_SIZE], end_idx: 0, tx: tx }
    }

    /// 쓰기 작업을 위한 stream 복사
    /// return: Option<TcpStream>
    pub fn get_write_stream(&self) -> Option<TcpStream> {
        match self.stream.try_clone() {
            Ok(stream) => { Some(stream) },
            _ => { None }
        }
    }

    /// read buffer에 도착한 데이터를 읽어서 적합한 작업 수행
    /// return: 같은 작업이 계속 필요한지에 대한 bool값
    pub fn read_message(&mut self) -> bool {
        match self.read_stream() {
            // 완성된 메시지를 조합할 수 있으면 출력하거나 이를 처리할 수 있는 스레드로 전달
            MessageStatus::Message(header, message) => {
                match self.tx {
                    Some(ref mut _tx) => {
                        _tx.send(Signal::NewMessage(header, message));
                    },
                    _ => {
                        println!("{} >>> {}", header.sender_id, message);
                    }
                }
                true
            },
            // 도착한 데이터는 있지만 아직 완성되지 않았으므로 다음 호출에서 처리
            MessageStatus::NoMessage => { true },
            // 클라이언트 접속이 끊어졌거나 클라이언트에서 접속 종료 요청을 보낸 경우 해당 이벤트를 전송하고 스레드 종료
            MessageStatus::Broken => { 
                match self.tx {
                    Some(ref mut _tx) => {
                        _tx.send(Signal::Close(self.id));
                    },
                    _ => {},
                }
                false 
            },
        }
    }

    /// read buffer에서 데이터를 읽고 조합, 버퍼 관리 수행 
    /// return: 바이트 스트림 데이터를 메시지로 조합한 결과 MessageStatus(enum)
    pub fn read_stream(&mut self) -> MessageStatus {
        match self.stream.read(&mut self.read_buf) {
            Ok(recv_size) => {
                if recv_size == 0 { return MessageStatus::Broken; }

                // read buffer에 있는 데이터를 recv buffer로 복사
                for idx in 0..recv_size {
                    self.recv_buf[self.end_idx] = self.read_buf[idx];
                    self.end_idx += 1;
                }

                // 메시지 조합
                let (body_end, header, message) = self.handle_received_message();
                    
                match body_end {
                    // 읽은 데이터가 유효하지 않은 상태
                    CLOSE_CODE => { return MessageStatus::Broken; },
                    // 아직 모든 메시지 데이터가 도착하지 않은 상태
                    0 => { return MessageStatus::NoMessage; },
                    // 조합된 메시지를 반환
                    _ => {
                        // 완성된 메시지만큼 recv buffer에서 삭제 
                        for idx in 0..(self.end_idx-body_end) {
                            self.recv_buf[idx] = self.recv_buf[body_end+idx];
                        }
                        
                        self.end_idx -= body_end;
                    
                        match message {
                            Some(chat_message) => {
                                MessageStatus::Message(header.unwrap(), chat_message)
                            },
                            None => { MessageStatus::NoMessage },
                        }
                    }
                }
            },
            Err(_) => { 
                println!("[DEBUG] read stream error");
                return MessageStatus::Broken; 
            }
        }
    }

    /// read buffer에 도착한 데이터를 완성된 형태의 메시지로 변환
    /// return: 조합된 메시지 메시지 마지막 idx, 메시지 헤더, 메시지 본문 (tuple)
    fn handle_received_message(&mut self) ->  (usize, Option<Header>, Option<String>) {
        let header: Header = decode_header(&self.recv_buf[0..HEADER_SIZE]);
        let body_end = (header.length + HEADER_SIZE) as usize;
        
        // 아직 본문 데이터가 모두 도착하지 않음 
        if self.end_idx < body_end { return (0, None, None); }
        
        match header.message_tag {
            // id 발급 메시지
            MESSAGE_TAG_ISUUE_ID => {
                self.id = header.sender_id;
                (body_end, Some(header), None)
            },
            // 채팅 메시지
            MESSAGE_TAG_CHAT => {
                let message = String::from_utf8_lossy(&self.recv_buf[HEADER_SIZE..body_end]);
                (body_end, Some(header), Some(message.to_string()))
            },
            // 접속 종료 요청 메시지
            MESSAGE_TAG_CLOSE => {
                println!("close the client please...");
                (CLOSE_CODE, None, None)
            },
            _ => { (body_end, None, None) },
        }
    }
}

pub fn read_loop(mut client: Client) {
    loop {
        if client.read_message() { continue; }
        else { break; }
    }
}

