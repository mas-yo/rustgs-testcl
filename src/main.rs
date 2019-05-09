use tokio::io;
use tokio::net::{TcpStream,TcpListener};
use tokio::prelude::*;
use std::net::SocketAddr;
use tokio::codec::{Decoder, Encoder, Framed};
use rand;
use rand::Rng;
use std::sync::{Arc,RwLock};
use std::env;
use uuid::Uuid;

mod command;

use crate::command::*;

fn start_client(addr: &str, id:u32) {
    let addr = format!("{}:18290", addr).parse().unwrap();

    let mut rng = rand::thread_rng();
    let name:String = Uuid::new_v4().to_hyphenated().to_string();
    let bye_text = format!("bye {}", name);

//    let name = format!("{}", name);
    let client = TcpStream::connect(&addr).and_then(move|stream| {
        let framed = Framed::new(stream, Codec::default());
        let (tx,rx) = framed.split();
        let mut tx = tx.wait();
        let receive = rx.take_while(move|cmd|{
            // println!("recv {:?}", cmd);
            match cmd {
                S2C::ShowUI(uiid,show) if *uiid==1001 => {
                    tx.send(C2S::TouchUI(1001)).unwrap();
                    tx.flush();
                },
                S2C::RequestLoginInfo => {
                    tx.send(C2S::ResponseLoginInfo(name.clone())).unwrap();
                    tx.flush();
                },
                S2C::ShowUI(uiid,show) if *uiid==2 => {
                    // println!("recv show ui 2");
                    for i in 0..10 { //ここが大きすぎると返事がこない
                        let mut text = (0..10).map(|_|"X").collect::<String>();
                        // text.insert_str(0, &text.clone());
                        // text.insert_str(0, &text.clone());
                        // text.insert_str(0, &text.clone());
                        // text.insert_str(0, &text.clone());
                        tx.send(C2S::InputText(text)).unwrap();
                        tx.flush();
                    }
                    tx.send(C2S::InputText(bye_text.clone())).unwrap();
                    tx.flush();
                    println!("send bye");
                    // return Ok(false);
                },
                S2C::AddText(uiid,text) => {
                    // println!("{:?} recv characters: {}", std::time::SystemTime::now(), text.len(), );
                    if text.ends_with(&bye_text) {
                        println!("disconnect. {}", id);
                        return Ok(false);
                    }
                }
                _ => {}
            }
            Ok(true)
        }).for_each(|_|Ok(()))
        .map_err(|_|());
        tokio::spawn(receive);
        Ok(())
    })
    .map_err(|err| {
        println!("connection error = {:?}", err);
    });

    println!("start client {}", id);
    tokio::spawn(client);
}

pub fn main() -> Result<(), Box<std::error::Error>> {

    let args:Vec<String> = env::args().collect();
    let task = Ok(()).into_future().and_then(move|_|{
        let addr = &args[1];
        for i in 0..500 {
            start_client(addr, i);
        }
        Ok(())
    });
    // .map_err(|_|());

    tokio::run(task);

    Ok(())
}
