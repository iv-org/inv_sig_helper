mod consts;
mod jobs;

use consts::DEFAULT_SOCK_PATH;
use jobs::JobOpcode;
use std::env::args;
use tokio::{
    io::{AsyncReadExt, BufReader},
    net::{UnixListener, UnixStream},
};

#[tokio::main]
async fn main() {
    let args: Vec<String> = args().collect();
    let socket_url: &String = args.get(1).unwrap_or(&DEFAULT_SOCK_PATH);

    let socket = UnixListener::bind(socket_url).unwrap();

    loop {
        let (socket, _addr) = socket.accept().await.unwrap();

        tokio::spawn(async move {
            process_socket(socket);
        });
    }
}

async fn process_socket(socket: UnixStream) {
    let mut bufreader = BufReader::new(socket);

    let opcode_byte: u8 = bufreader.read_u8().await.unwrap();
    let opcode: JobOpcode = opcode_byte.try_into().unwrap();

    match opcode {
        JobOpcode::ForceUpdate => {}
        JobOpcode::DecryptNSignature => {}
        _ => {}
    }
}
