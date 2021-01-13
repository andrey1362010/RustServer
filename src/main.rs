mod server;

use crate::server::run_epoll_server;

fn main() {
    let address = "127.0.0.1:13265";
    let num_threads = 4;
    run_epoll_server(address, num_threads, |data| server::RESPONSE.to_string());
}