extern crate bincode;
extern crate rustc_serialize;
extern crate client;
extern crate header;

use std::io;
use std::io::prelude::*;
use std::net::TcpStream;
use std::thread;
use client::*;
use header::*;

const COMMAND_TERMINATION: &'static str = "/exit\n";

fn main() {
    // 서버로의 tcp connection 생성 
    let new_stream = TcpStream::connect("127.0.0.1:9000").unwrap();
    
    println!("connected");
    
    // 서버가 전송하는 메시지들을 처리해서 화면에 보여주는 작업을 처리하기 위한 객체 생성
    let client = Client::new(new_stream, None);

    // 사용자의 입력을 처리해서 전송하기 위한 stream 생성 
    let write_stream = client.get_write_stream().unwrap();
    let read_thread = thread::spawn(move|| { 
        read_loop(client);
    });
    
    loop {
        let mut user_input: String = String::new();
	    io::stdin().read_line(&mut user_input);
        
        // 종료 커맨드("/exit")를 입력하면 프로그램 종료 
        if user_input == COMMAND_TERMINATION {
            println!("exit....");
            send_message(&write_stream, &Header::new(0, MESSAGE_TAG_CLOSE, 0), None);
            break;
        }

        // 메시지 전송 
        send_message(&write_stream, &Header::new(user_input.len(), MESSAGE_TAG_CHAT, 0), Some(user_input));
    }
    
    read_thread.join();
}