use mio::net::{TcpListener, TcpStream};
use mio::{Events, Poll, Token, Interest};
use std::io::{Write, Read};
use std::error::Error;
use std::{thread, time};
use std::collections::HashMap;
use std::sync::{Mutex, Arc};
use std::rc::Rc;
use threadpool::ThreadPool;
use std::net::SocketAddr;
use std::cell::RefCell;
use std::str::from_utf8;
use std::borrow::Borrow;

const NUM_THREADS: usize = 4;
const SERVER: Token = Token(0);
const REQUEST_BUFFER_SIZE: usize = 8192;
const DEFAULT_RESPONSE: &str = "HTTP/1.1 200 OK\r\nContent-Length: 5\r\nConnection: Keep-Alive\r\n\r\nhello";

fn main() {
    println!("{}", "Teeeeesst");

    //let addr = "127.0.0.1:13265".parse().unwrap();
    let addr: SocketAddr = ([127, 0, 0, 1], 13265).into();
    println!("{}", "222");
    let mut poll = Poll::new().unwrap();
    println!("{}", "3333");
    let mut server = TcpListener::bind(addr).unwrap();
    let mut connections: HashMap<usize, TcpStream> = HashMap::new();

    println!("{}", "111");
    poll.registry().register(&mut server, SERVER, Interest::READABLE).unwrap();
    //poll.register(&mut server, SERVER, Ready::readable(), PollOpt::edge()).unwrap();
    println!("{}", "Server created");
    let mut events = Events::with_capacity(128);
    let mut buffer: [u8; REQUEST_BUFFER_SIZE] = [0; REQUEST_BUFFER_SIZE];
    loop {
        //events.clear();
        poll.poll(&mut events, None).unwrap();
        // println!("{}", "Events polled");
        for event in events.iter() {
            match event.token() {
                SERVER => {
                    //println!("{}", "Process -> Server...");
                    loop {
                        match server.accept() {
                            Ok((mut stream, address)) => {
                                stream.set_nodelay(true).unwrap();
                                let token_id = address.port() as usize;
                                let token = Token(token_id);
                                poll.registry().register(&mut stream, token, Interest::READABLE).unwrap();
                                //let stream = Rc::new(RefCell::new(stream));
                                connections.insert(token_id, stream);
                                println!("new");
                            }
                            Err(E) => {
                                println!("Error Server {}", E);
                                break;
                            }
                        }
                    }
                    poll.registry().reregister(&mut server, SERVER, Interest::READABLE).unwrap();
                }

                Token(token_id) => {
                    if event.is_readable() {
                        //println!("{}", "Process -> Client...");
                        let mut local_stream = connections.get_mut(&token_id).unwrap();
                        let mut need_write:bool = false;
                        loop {
                            match local_stream.read(&mut buffer) {
                                Ok(0) => {
                                    //println!("recieved empty...");
                                    //connections.remove(&token_id).unwrap();
                                    //need_write = false;
                                    break;
                                }
                                Ok(n) => {
                                    //println!("{}", from_utf8(&buffer).unwrap())
                                    need_write = true;
                                }
                                Err(E) => {
                                    println!("Error Client {}", E);
                                    //need_write = false;
                                    break;
                                }
                            }
                        }
                        if need_write {
                            local_stream.write(DEFAULT_RESPONSE.as_bytes());
                            local_stream.flush().unwrap();
                            poll.registry().reregister(local_stream, event.token(), Interest::READABLE).unwrap();
                        }

                    }
                }
            }
        }
    }

    return ();
}

