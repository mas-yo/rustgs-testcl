use tokio::io;
use tokio::net::{TcpStream,TcpListener};
use tokio::prelude::*;
use std::net::SocketAddr;
use tokio::codec::{Decoder, Encoder, Framed};
use rand;
use rand::Rng;
use std::sync::{Arc,RwLock,Mutex};
use std::env;
use uuid::Uuid;
use lazy_static::lazy_static;

mod command;

use crate::command::*;

lazy_static! {
    static ref CONNS_COUNT : Arc<Mutex<u32>> = {
        Arc::new(Mutex::new(0u32))
    };
}
lazy_static! {
    static ref TIME_SUM : Arc<Mutex<u128>> = {
        Arc::new(Mutex::new(0u128))
    };
}

fn start_client(addr: String, id:u32, name: String, tx_next: futures::sync::mpsc::Sender<(u32,String)>) {
    let addr = format!("{}:18290", addr).parse().unwrap();
    let name2 = name.clone();

    // let mut rng = rand::thread_rng();
    // let name:String = Uuid::new_v4().to_hyphenated().to_string();
    let bye_text = format!("bye {}", name);

//    let name = format!("{}", name);
    let client = TcpStream::connect(&addr).and_then(move|stream| {
        stream.set_nodelay(true);
        println!("connected");
        let framed = Framed::new(stream, Codec::default());
        let (tx,rx) = framed.split();
        let mut tx = tx.wait();
        let mut send_at = None;

        let receive = rx.take_while(move|cmd|{

            // println!("recv {:?}", cmd);
            match cmd {
                S2C::ShowUI(uiid,show) if *uiid==1001 => {
                    if let Err(e) = tx.send(C2S::TouchUI(1001)) {
                        println!("disconnect. {} err {}", id, e);
                        return Ok(false);
                    }
                    if let Err(e) = tx.flush() {
                        println!("disconnect. {} err {}", id, e);
                        return Ok(false);
                    }
                },
                S2C::RequestLoginInfo => {
                    // println!("request login");
                    if let Err(e) = tx.send(C2S::ResponseLoginInfo(name.clone())) {
                        println!("disconnect. {} err {}", id, e);
                        return Ok(false);
                    }
                    if let Err(e) = tx.flush() {
                        println!("disconnect. {} err {}", id, e);
                        return Ok(false);
                    }
                },
                S2C::ShowUI(uiid,show) if *uiid==2 => {
                    println!("recv show ui 2");
                    for _ in 0..1 { //ここが大きすぎると返事がこない
                        let text = (0..10).map(|_|"X").collect::<String>();
                        // text.insert_str(0, &text.clone());
                        // text.insert_str(0, &text.clone());
                        // text.insert_str(0, &text.clone());
                        // text.insert_str(0, &text.clone());
                        if let Err(e) = tx.send(C2S::InputText(text)) {
                            println!("disconnect. {} err {}", id, e);
                            return Ok(false);
                        }
                        println!("send msg");
                        if let Err(e) = tx.flush() {
                            println!("disconnect. {} err {}", id, e);
                            return Ok(false);
                        }
                        println!("flushed");
                    }
                    tx.send(C2S::InputText(bye_text.clone())).unwrap();
                    println!("send bye");
                    if let Err(e) = tx.flush() {
                        println!("disconnect. {} err {}", id, e);
                        return Ok(false);
                    }
                    println!("flushed bye");
                    send_at = Some(std::time::SystemTime::now());
                    // println!("send bye");
                    // return Ok(false);
                },
                S2C::AddText(uiid,text) => {
                    println!("{:?} recv characters: {}", std::time::SystemTime::now(), text.len(), );
                    if text.ends_with(&bye_text) {
                        let mut t: u128 = 0;
                        if let Some(at) = send_at {
                            let time = at.elapsed().unwrap();
                            t = time.as_millis();
                            {
                                let mut lock = CONNS_COUNT.lock().unwrap();
                                *lock += 1;
                            }
                            {
                                let mut lock = TIME_SUM.lock().unwrap();
                                *lock += t;
                            }
                        }
                        println!("disconnect. {} time {}", id, t);
                        std::thread::sleep(std::time::Duration::from_millis(10));
                        return Ok(false);
                    }
                }
                _ => {}
            }
            Ok(true)
        }).for_each(|_|Ok(()))
        .map_err(|_|())
        .then(move|_|{
            if let Err(e) = tx_next.wait().send((id, name2)) {
                println!("room nexe send err {}", e);
            }
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

    let (tx,rx) = futures::sync::mpsc::channel::<(u32,String)>(1024);
    let tx2 = tx.clone();

    let mut ids = Vec::new();
    for i in 0..100 {
        ids.push(format!("user{:>04}", i));
    }

    let mut wait = tx.wait();
    for i in 0..50 {
        if let Err(e) = wait.send((i, ids[i as usize].clone())) {
            println!("first send room err {}", e);
        }
    }
    // .map_err(|_|());

    let report_time = Ok(()).into_future().and_then(|_|{
        let task = tokio::timer::Interval::new(std::time::Instant::now(), std::time::Duration::from_secs(10))
        .for_each(|_|{
            let time = TIME_SUM.lock().unwrap();
            let count = CONNS_COUNT.lock().unwrap();
            println!("avg time: {}", *time as f64 / *count as f64);
            Ok(())
        })
        .map_err(|_|());
        // tokio::spawn(task);
        Ok(())
    });
    {
        let start = rx.for_each(move|(id,name)| {
            start_client(addr.clone(), id, name, tx2.clone());
            Ok(())
        }).and_then(|_|{
            println!("END");Ok(())
        }); 
        tokio::run(report_time.and_then(|_| start));
    }

    Ok(())
}
