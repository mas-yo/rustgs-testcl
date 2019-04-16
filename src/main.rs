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

fn start_client(addr: &str) {
    let addr = format!("{}:18290", addr).parse().unwrap();

    let mut rng = rand::thread_rng();
    let name:String = Uuid::new_v4().to_hyphenated().to_string();
//    let name = format!("{}", name);
    let client = TcpStream::connect(&addr).and_then(move|stream| {
        let framed = Framed::new(stream, Codec::default());
        let (tx,rx) = framed.split();
        let mut opt_tx = Some(tx);
        let receive = rx.take_while(move|cmd|{
            println!("{:?}", cmd);
            match cmd {
                S2C::ShowUI(uiid,show) if *uiid==1001 => {
                    let tx = opt_tx.take().unwrap();
                    opt_tx = Some(tx.send(C2S::TouchUI(1001)).wait().unwrap());
                },
                S2C::RequestLoginInfo => {
                    let tx = opt_tx.take().unwrap();
                    opt_tx = Some(tx.send(C2S::ResponseLoginInfo(name.clone()))).wait().unwrap();
                },
                S2C::ShowUI(uiid,show) if *uiid==2 => {
                    for i in 0..1000 { //ここが大きすぎると返事がこない
                        let mut text = (0..100).map(|_|"X").collect::<String>();
                        // text.insert_str(0, &text.clone());
                        // text.insert_str(0, &text.clone());
                        // text.insert_str(0, &text.clone());
                        // text.insert_str(0, &text.clone());
                        let tx = opt_tx.take().unwrap();
                        opt_tx = Some(tx.send(C2S::InputText(text)).wait().unwrap());
                    }
                    let tx = opt_tx.take().unwrap();
                    opt_tx = Some(tx.send(C2S::InputText("bye".to_string())).wait().unwrap());
                    // return Ok(false);
                },
                S2C::AddText(uiid,text) => {
                    println!("recv characters: {}", text.len());
                    if str::ends_with(text, "bye") {
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

    tokio::spawn(client);
}

pub fn main() -> Result<(), Box<std::error::Error>> {

    let args:Vec<String> = env::args().collect();
    let task = Ok(()).into_future().and_then(move|_|{
        let addr = &args[1];
        for _ in 0..1 {
            start_client(addr);
        }
        Ok(())
    });
    // .map_err(|_|());

    tokio::run(task);

    Ok(())
}
