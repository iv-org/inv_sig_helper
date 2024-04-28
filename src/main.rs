mod consts;
mod jobs;

use consts::DEFAULT_SOCK_PATH;
use jobs::{process_decrypt_n_signature, process_fetch_update, GlobalState, JobOpcode};
use std::{env::args, sync::Arc};
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

    // have to please rust
    let state: Arc<GlobalState> = Arc::new(GlobalState::new());

    let socket = UnixListener::bind(socket_url).unwrap();

    loop {
        let (socket, _addr) = socket.accept().await.unwrap();

        let cloned_state = state.clone();
        tokio::spawn(async {
            process_socket(cloned_state, socket).await;
        });
    }
}

async fn process_socket(state: Arc<GlobalState>, socket: UnixStream) {
    let mut bufreader = BufReader::new(socket);

    loop {
        let opcode_byte: u8 = break_fail!(bufreader.read_u8().await);
        let opcode: JobOpcode = opcode_byte.into();

        match opcode {
            JobOpcode::ForceUpdate => {
                let cloned_state = state.clone();
                tokio::spawn(async {
                    process_fetch_update(cloned_state).await;
                });
            }
            JobOpcode::DecryptNSignature => {
                let sig_size: usize = usize::from(break_fail!(bufreader.read_u16().await));
                let mut buf = vec![0u8; sig_size];

                break_fail!(bufreader.read_exact(&mut buf).await);

                let str = break_fail!(String::from_utf8(buf));
                let cloned_state = state.clone();
                tokio::spawn(async {
                    process_decrypt_n_signature(cloned_state, str).await;
                });
            }
            _ => {}
        }
    }
}
