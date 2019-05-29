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

fn start_client(addr: String, id:u32, name: String, tx_next: futures::sync::mpsc::Sender<(u32,String)>) {
    let addr = format!("{}:18290", addr).parse().unwrap();
    let name2 = name.clone();

    // let mut rng = rand::thread_rng();
    // let name:String = Uuid::new_v4().to_hyphenated().to_string();
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
                    if let Err(e) = tx.flush() {
                        println!("disconnect. {} err {}", id, e);
                        return Ok(false);
                    }
                },
                S2C::RequestLoginInfo => {
                    println!("request login");
                    tx.send(C2S::ResponseLoginInfo(name.clone())).unwrap();
                    if let Err(e) = tx.flush() {
                        println!("disconnect. {} err {}", id, e);
                        return Ok(false);
                    }
                },
                S2C::ShowUI(uiid,show) if *uiid==2 => {
                    println!("recv show ui 2");
                    for i in 0..10 { //ここが大きすぎると返事がこない
                        let mut text = (0..10).map(|_|"X").collect::<String>();
                        // text.insert_str(0, &text.clone());
                        // text.insert_str(0, &text.clone());
                        // text.insert_str(0, &text.clone());
                        // text.insert_str(0, &text.clone());
                        tx.send(C2S::InputText(text)).unwrap();
                        if let Err(e) = tx.flush() {
                            println!("disconnect. {} err {}", id, e);
                            return Ok(false);
                        }
                    }
                    tx.send(C2S::InputText(bye_text.clone())).unwrap();
                    if let Err(e) = tx.flush() {
                        println!("disconnect. {} err {}", id, e);
                        return Ok(false);
                    }
                    // println!("send bye");
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
        .map_err(|_|())
        .then(move|_|{
            tx_next.wait().send((id, name2));
            Ok(())
        });

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
    let addr = args[1].clone();

    let (tx,rx) = futures::sync::mpsc::channel::<(u32,String)>(32);
    let tx2 = tx.clone();

    let mut ids = Vec::new();
    for i in 0..100 {
        ids.push(format!("user{:>04}", i));
    }

    let mut wait = tx.wait();
    for i in 0..1 {
        wait.send((i, ids[i as usize].clone()));
    }
    // .map_err(|_|());

    {
        let start = rx.for_each(move|(id,name)| {
            start_client(addr.clone(), id, name, tx2.clone());
            Ok(())
        }); 
        tokio::run(start);
    }

    Ok(())
}
