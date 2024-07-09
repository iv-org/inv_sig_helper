mod consts;
mod jobs;
mod opcode;
mod player;

use ::futures::StreamExt;
use consts::DEFAULT_SOCK_PATH;
use jobs::{process_decrypt_n_signature, process_fetch_update, GlobalState, JobOpcode};
use opcode::OpcodeDecoder;
use player::fetch_update;
use std::{env::args, sync::Arc};
use tokio::{
    fs::remove_file,
    net::{UnixListener, UnixStream},
    sync::Mutex,
};
use tokio_util::codec::Framed;

use crate::jobs::{
    process_decrypt_signature, process_get_signature_timestamp, process_player_status,
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

macro_rules! eof_fail {
    ($res:expr, $stream:ident) => {
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

    let socket: UnixListener = match UnixListener::bind(socket_url) {
        Ok(x) => x,
        Err(x) => {
            if x.kind() == std::io::ErrorKind::AddrInUse {
                remove_file(socket_url).await;
                UnixListener::bind(socket_url).unwrap()
            } else {
                println!("Error occurred while trying to bind: {}", x);
                return;
            }
        }
    };

    println!("Fetching player");
    match fetch_update(state.clone()).await {
        Ok(()) => println!("Successfully fetched player"),
        Err(x) => {
            println!("Error occured while trying to fetch the player: {:?}", x);
        }
    }
    loop {
        let (socket, _addr) = socket.accept().await.unwrap();

        let cloned_state = state.clone();
        tokio::spawn(async move {
            process_socket(cloned_state, socket).await;
        });
    }
}

async fn process_socket(state: Arc<GlobalState>, socket: UnixStream) {
    let decoder = OpcodeDecoder {};
    let str = Framed::new(socket, decoder);

    let (sink, mut stream) = str.split();

    let arc_sink = Arc::new(Mutex::new(sink));
    while let Some(opcode_res) = stream.next().await {
        match opcode_res {
            Ok(opcode) => {
                println!("Received job: {}", opcode.opcode);

                match opcode.opcode {
                    JobOpcode::ForceUpdate => {
                        let cloned_state = state.clone();
                        let cloned_sink = arc_sink.clone();
                        tokio::spawn(async move {
                            process_fetch_update(cloned_state, cloned_sink, opcode.request_id)
                                .await;
                        });
                    }
                    JobOpcode::DecryptNSignature => {
                        let cloned_state = state.clone();
                        let cloned_sink = arc_sink.clone();
                        tokio::spawn(async move {
                            process_decrypt_n_signature(
                                cloned_state,
                                opcode.signature,
                                cloned_sink,
                                opcode.request_id,
                            )
                            .await;
                        });
                    }
                    JobOpcode::DecryptSignature => {
                        let cloned_state = state.clone();
                        let cloned_sink = arc_sink.clone();
                        tokio::spawn(async move {
                            process_decrypt_signature(
                                cloned_state,
                                opcode.signature,
                                cloned_sink,
                                opcode.request_id,
                            )
                            .await;
                        });
                    }
                    JobOpcode::GetSignatureTimestamp => {
                        let cloned_state = state.clone();
                        let cloned_sink = arc_sink.clone();
                        tokio::spawn(async move {
                            process_get_signature_timestamp(
                                cloned_state,
                                cloned_sink,
                                opcode.request_id,
                            )
                            .await;
                        });
                    }
                    JobOpcode::PlayerStatus => {
                        let cloned_state = state.clone();
                        let cloned_sink = arc_sink.clone();
                        tokio::spawn(async move {
                            process_player_status(cloned_state, cloned_sink, opcode.request_id)
                                .await;
                        });
                    }
                    _ => {
                        continue;
                    }
                }
            }
            Err(x) => {
                println!("I/O error: {:?}", x);
                break;
            }
        }
    }
}
