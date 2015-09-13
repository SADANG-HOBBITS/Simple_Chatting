extern crate bincode;
extern crate rustc_serialize;
extern crate client;
extern crate header;

use std::net::TcpStream;
use std::sync::mpsc;
use std::sync::mpsc::{Sender, Receiver};
use std::thread;
use std::collections::HashMap;
use client::*;
use header::*;

pub struct EventHandler {
    pub client_stream_map: HashMap<u64, TcpStream>,
    pub last_issued_client_id: u64,
    pub tx: Sender<Signal>,
    pub rx: Receiver<Signal>
}

impl EventHandler {
    pub fn new() -> EventHandler {
        let (_tx, _rx): (Sender<Signal>, Receiver<Signal>) = mpsc::channel();
        EventHandler{ client_stream_map: HashMap::new(), last_issued_client_id: 0, tx: _tx, rx: _rx }
    }

    /// 외부 스레드에서 발생한 이벤트를 EventHandler 스레드로 전송하기 위한 channel(Send) 복사
    /// return: Option<Sender<Signal>>
    pub fn get_transmitter(&self) -> Option<Sender<Signal>> {
        Some(self.tx.clone())
    }

    /// channel(Receiver)에 등록된 메시지가 있으면, 해당 메시지에 맞는 작업 수행
    /// channel 추가 정보: https://doc.rust-lang.org/stable/book/concurrency.html
    pub fn cycle(&mut self) {
        loop {
            match self.rx.recv().unwrap() {
                // 새로운 클라이언트가 접속한 경우
                Signal::NewClient(mut new_client) => {
                    self.last_issued_client_id += 1;

                    // id를 발급하고, stream을 복사해서 메시지 전송용으로 사용
                    new_client.id = self.last_issued_client_id;
                    let client_stream = new_client.get_write_stream().unwrap();
                    send_message(&client_stream, &Header::new(0, MESSAGE_TAG_ISUUE_ID, self.last_issued_client_id), None);

                    // 해당 클라이언트가 보내는 메시지를 읽는 스레드 생성
                    self.client_stream_map.insert(self.last_issued_client_id, client_stream);
                    thread::spawn(move|| {
                        read_loop(new_client);
                    });
                },
                // 새로운 채팅 메시지가 도착한 경우
                Signal::NewMessage(new_header, new_message) => {
                    // 등록된 클라이언트를 순회하면서 발송
                    for (id, each_client_stream) in &mut self.client_stream_map {
                        send_message(&each_client_stream, &new_header, Some(new_message.clone()));
                    }
                },
                // 접속 종료한 클라이언트를 방송 대상에서 제외
                Signal::Close(id) => {
                    send_message(&self.client_stream_map.get(&id).unwrap(), &Header::new(0, MESSAGE_TAG_CLOSE, id), None);
                    self.client_stream_map.remove(&id);
                }
            }
        }
    }
}

pub fn handle_event_handler(mut event_handler: EventHandler) {
    event_handler.cycle();
}