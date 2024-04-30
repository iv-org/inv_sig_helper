mod consts;
mod jobs;

use consts::DEFAULT_SOCK_PATH;
use jobs::{process_decrypt_n_signature, process_fetch_update, GlobalState, JobOpcode};
use std::{env::args, io::Error, sync::Arc};
use tokio::{
    io::{self, AsyncReadExt, BufReader, BufWriter},
    net::{UnixListener, UnixStream},
    sync::Mutex,
};

use crate::jobs::{process_decrypt_signature, process_get_signature_timestamp};

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

macro_rules! eof_fail {
    ($res:expr, $stream:ident) => {
        match $res {
            Ok(value) => value,
            Err(e) => {
                if (e.kind() == io::ErrorKind::UnexpectedEof) {
                    $stream.get_ref().readable().await?;
                    continue;
                }
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

async fn process_socket(state: Arc<GlobalState>, socket: UnixStream) -> Result<(), Error> {
    let (rd, wr) = socket.into_split();

    let wrapped_readstream = Arc::new(Mutex::new(BufReader::new(rd)));
    let wrapped_writestream = Arc::new(Mutex::new(BufWriter::new(wr)));

    let cloned_readstream = wrapped_readstream.clone();
    let mut inside_readstream = cloned_readstream.lock().await;

    loop {
        inside_readstream.get_ref().readable().await?;

        let cloned_writestream = wrapped_writestream.clone();

        let opcode_byte: u8 = eof_fail!(inside_readstream.read_u8().await, inside_readstream);
        let opcode: JobOpcode = opcode_byte.into();
        let request_id: u32 = eof_fail!(inside_readstream.read_u32().await, inside_readstream);

        println!("Received job: {}", opcode);
        match opcode {
            JobOpcode::ForceUpdate => {
                let cloned_state = state.clone();
                let cloned_stream = cloned_writestream.clone();
                tokio::spawn(async move {
                    process_fetch_update(cloned_state, cloned_stream, request_id).await;
                });
            }
            JobOpcode::DecryptNSignature => {
                let sig_size: usize = usize::from(eof_fail!(
                    inside_readstream.read_u16().await,
                    inside_readstream
                ));
                let mut buf = vec![0u8; sig_size];

                break_fail!(inside_readstream.read_exact(&mut buf).await);

                let str = break_fail!(String::from_utf8(buf));
                let cloned_state = state.clone();
                let cloned_stream = cloned_writestream.clone();
                tokio::spawn(async move {
                    process_decrypt_n_signature(cloned_state, str, cloned_stream, request_id).await;
                });
            }
            JobOpcode::DecryptSignature => {
                let sig_size: usize = usize::from(eof_fail!(
                    inside_readstream.read_u16().await,
                    inside_readstream
                ));
                let mut buf = vec![0u8; sig_size];

                break_fail!(inside_readstream.read_exact(&mut buf).await);

                let str = break_fail!(String::from_utf8(buf));
                let cloned_state = state.clone();
                let cloned_stream = cloned_writestream.clone();
                tokio::spawn(async move {
                    process_decrypt_signature(cloned_state, str, cloned_stream, request_id).await;
                });
            }
            JobOpcode::GetSignatureTimestamp => {
                let cloned_state = state.clone();
                let cloned_stream = cloned_writestream.clone();
                tokio::spawn(async move {
                    process_get_signature_timestamp(cloned_state, cloned_stream, request_id).await;
                });
            }
            _ => {}
        }
    }

    Ok(())
}
