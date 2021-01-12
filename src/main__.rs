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
use std::str::from_utf8;

const NUM_THREADS: usize = 4;
const SERVER: Token = Token(0);
const REQUEST_BUFFER_SIZE:usize = 8192;
const DEFAULT_RESPONSE: &str = "HTTP/1.1 200 OK\r\nContent-Length: 5\r\nConnection: Keep-Alive\r\n\r\nhello";

fn main() {


    println!("{}", "Teeeeesst");

    let addr = "127.0.0.1:13265".parse().unwrap();
    let mut poll = Poll::new().unwrap();
    let mut server = TcpListener::bind(addr).unwrap();
    let mut connections:HashMap<usize, Arc<Mutex<TcpStream>>> = HashMap::new();
    let server_thread_pool = ThreadPool::with_name("server_pool".to_string(), NUM_THREADS);

    poll.registry().register(&mut server, SERVER, Interest::READABLE).unwrap();
    let mut events = Events::with_capacity(128);
    println!("{}", "Server created");

    loop {
        events.clear();
        poll.poll(&mut events, None).unwrap();
        for event in events.iter() {
            match event.token() {
                SERVER => {
                    let (mut stream, address) = server.accept().unwrap();
                    stream.set_nodelay(true).unwrap();
                    let token_id = address.port() as usize;
                    let token = Token(token_id);
                    poll.registry().register(&mut stream, token, Interest::READABLE).unwrap();
                    poll.registry().reregister(&mut server, SERVER, Interest::READABLE).unwrap();
                    let stream = Arc::new(Mutex::new(stream));
                    connections.insert(token_id, stream.clone());
                }
                Token(token_id) => {
                    let mut buffer:[u8; REQUEST_BUFFER_SIZE] = [0; REQUEST_BUFFER_SIZE];
                    let mut local_stream = connections.get(&token_id).unwrap().clone();
                    let mut locked_stream = local_stream.lock().unwrap();
                    match locked_stream.read(&mut buffer) {
                        Ok(0) => {}
                        Ok(n) => {
                            //println!("{}", from_utf8(&buffer).unwrap());
                            poll.registry().reregister(&mut *locked_stream, event.token(), Interest::READABLE).unwrap();
                            let mut local_stream = local_stream.clone();
                            server_thread_pool.execute(move || {
                                //println!("Thread:{}", thread::current());
                                let mut locked_stream = local_stream.lock().unwrap();
                                locked_stream.write(DEFAULT_RESPONSE.as_bytes());
                                locked_stream.flush().unwrap();
                            });
                        }
                        Err(E) => {}
                    }
                }
            }
        }
    }

   return ()
}

