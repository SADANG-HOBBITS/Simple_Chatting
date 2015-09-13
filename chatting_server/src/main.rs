extern crate event_handler;
extern crate client;

use std::net::TcpListener;
use std::thread;
use client::*;
use event_handler::*;

fn main() {
    // 클라이언트의 접속을 처리할 listen port 생성 
    let listener = TcpListener::bind("127.0.0.1:9000").unwrap();
    
    println!("Start to listen, ready to accept");

    // 각종 이벤트를 처리할 로직 생성 
    let sample_event_handler = EventHandler::new();

    // 클라이언트의 접속 이벤트를 전송하기 위한 channel(Send) 할당 
    let _tx = sample_event_handler.get_transmitter().unwrap();
    thread::spawn(move|| {
        handle_event_handler(sample_event_handler);
    });

    for stream in listener.incoming() {
        match stream {
            Ok(new_stream) => {
                // 새로운 클라이언트의 연결이 생기면 이벤트 처리 스레드로 이벤트 생성, 전달
                let new_client = Client::new(new_stream, Some(_tx.clone()));
                _tx.send(Signal::NewClient(new_client));
            }
            Err(_) => { println!("connection failed"); }
        }
    }
    
    drop(listener);
}