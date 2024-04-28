mod consts;
mod jobs;

use consts::DEFAULT_SOCK_PATH;
use jobs::{process_decrypt_n_signature, process_fetch_update, JobOpcode};
use std::env::args;
use tokio::{
    io::{AsyncReadExt, BufReader},
    net::{UnixListener, UnixStream},
};

macro_rules! break_fail {
    ($res:expr) => {
        match $res {
            Ok(value) => value,
            Err(e) => {
                println!("An error occurred while parsing the current request: {}", e);
                break;
            }
        }
    };
}

#[tokio::main]
async fn main() {
    let args: Vec<String> = args().collect();
    let socket_url: &str = match args.get(1) {
        Some(stringref) => stringref,
        None => DEFAULT_SOCK_PATH,
    };

    let socket = UnixListener::bind(socket_url).unwrap();

    loop {
        let (socket, _addr) = socket.accept().await.unwrap();

        tokio::spawn(async move {
            process_socket(socket).await;
        });
    }
}

async fn process_socket(socket: UnixStream) {
    let mut bufreader = BufReader::new(socket);

    loop {
        let opcode_byte: u8 = break_fail!(bufreader.read_u8().await);
        let opcode: JobOpcode = opcode_byte.into();

        match opcode {
            JobOpcode::ForceUpdate => {
                tokio::spawn(async move {
                    process_fetch_update().await;
                });
            }
            JobOpcode::DecryptNSignature => {
                let sig_size: usize = usize::from(break_fail!(bufreader.read_u16().await));
                let mut buf = vec![0u8; sig_size];

                break_fail!(bufreader.read_exact(&mut buf).await);

                let _str = break_fail!(String::from_utf8(buf));

                tokio::spawn(async move {
                    process_decrypt_n_signature(_str).await;
                });
            }
            _ => {}
        }
    }
}
