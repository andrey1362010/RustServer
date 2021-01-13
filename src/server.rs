use mio::net::{TcpListener, TcpStream};
use mio::{Poll, Token, Interest, Events};
use std::time::Duration;
use std::collections::HashMap;
use std::sync::mpsc::{channel, Receiver, Sender};
use threadpool::ThreadPool;
use std::sync::{Mutex, Arc};
use std::io::{Read, Write};

pub(crate) static RESPONSE: &str = "HTTP/1.1 200 OK\r\nContent-Length: 5\r\nConnection: Keep-Alive\r\n\r\nhello";


struct Handler {
    token: Token,
    socket: TcpStream,
    is_open: bool,
}

impl Handler {
    fn init(token: Token, socket: TcpStream) -> Handler {
        Handler {
            token,
            socket,
            is_open: true,
        }
    }

    fn read_from_socket(&mut self) -> Option<Vec<u8>> {
        let mut vec = Vec::with_capacity(1024);
        let mut buffer = [0 as u8; 1024];
        loop {
            let read = self.socket.read(&mut buffer);
            match read {
                Ok(0) => {
                    self.is_open = false;
                    return None
                }
                Ok(n) => {
                    for i in 0..n {
                        vec.push(buffer[i]);
                    }
                }
                _ => {
                    break;
                }
            }
        }
        if vec.len() == 0 {
             return None
        }
        Some(vec)
    }

    fn write_to_socket(&mut self, data: &str) {
        match self.socket.write_all(data.as_bytes()) {
            Ok(_) => (),
            Err(_) => {
                self.is_open = false;
                return;
            }
        }
    }

}

pub(crate) fn run_epoll_server(address:&str, num_threads:usize, process_function:fn(Vec<u8>) -> String) {
    let mut listener = TcpListener::bind(address.parse().unwrap()).unwrap();

    let mut poll = Poll::new().unwrap();
    poll.registry().register(&mut listener, Token(0), Interest::READABLE).unwrap();

    let mut handlers: HashMap<Token, Handler> = HashMap::new();
    let (tx, rx): (Sender<Handler>, Receiver<Handler>) = channel();
    let (ready_tx, ready_rx): (Sender<Handler>, Receiver<Handler>) = channel();
    let rx = Arc::new(Mutex::new(rx));

    let mut pool = ThreadPool::new(num_threads);
    for _ in 0..num_threads {
        let rx = Arc::clone(&rx);
        let ready_tx = ready_tx.clone();
        pool.execute(move || {
            loop {
                //println!("Try lock..!");
                let mut handler = rx.lock().unwrap().recv().unwrap();
                //println!("Process begin..!");
                let data = handler.read_from_socket();
                match data {
                    None => {}
                    Some(DATA) => {
                        let resp = process_function(DATA);
                        handler.write_to_socket(&resp);
                    }
                }
                //println!("Process end..!");
                ready_tx.send(handler).unwrap();
            }
        });
    }


    let mut events = Events::with_capacity(1024);
    loop {
        poll.poll(&mut events, Some(Duration::from_millis(5))).unwrap();
        for event in &events {
            match event.token() {
                Token(0) => {
                    loop {
                        match listener.accept() {
                            Ok((mut socket, address)) => {
                                let token_id = address.port() as usize;
                                let token = Token(token_id);
                                poll.registry().register(&mut socket, token, Interest::READABLE).unwrap();
                                handlers.insert(token, Handler::init(token, socket));
                                //println!("Connection accepted!")
                            }
                            Err(_) => break
                        }
                    }
                    poll.registry().reregister(&mut listener, Token(0), Interest::READABLE).unwrap();
                }
                token if event.is_readable() => {
                    if let Some(handler) = handlers.remove(&token) {
                        //println!("Connection send to read..!");
                        tx.send(handler).unwrap();
                    }
                }
                _ => unreachable!()
            }
        }

        loop {
            let opt = ready_rx.try_recv();
            match opt {
                Ok(mut handler) if handler.is_open => {
                    //println!("Recreate register!");
                    poll.registry().reregister(&mut handler.socket, handler.token, Interest::READABLE).unwrap();
                    handlers.insert(handler.token, handler);
                }
                _ => break,
            }
        }
    }
}
