mod consts;
mod jobs;
mod opcode;
mod player;

use ::futures::StreamExt;
use consts::{DEFAULT_SOCK_PATH, DEFAULT_TCP_URL};
use jobs::{process_decrypt_n_signature, process_fetch_update, GlobalState, JobOpcode};
use opcode::OpcodeDecoder;
use player::fetch_update;
use std::{env::args, sync::Arc};
use tokio::{
    fs::remove_file,
    io::{AsyncReadExt, AsyncWrite},
    net::{TcpListener, UnixListener},
    sync::Mutex,
};
use tokio_util::codec::Framed;

use crate::jobs::{
    process_decrypt_signature, process_get_signature_timestamp, process_player_status,
    process_player_update_timestamp,
};

macro_rules! loop_main {
    ($i:ident, $s:ident) => {
        println!("Fetching player");
        match fetch_update($s.clone()).await {
            Ok(()) => println!("Successfully fetched player"),
            Err(x) => {
                println!("Error occured while trying to fetch the player: {:?}", x);
            }
        }
        loop {
            let (socket, _addr) = $i.accept().await.unwrap();

            let cloned_state = $s.clone();
            tokio::spawn(async move {
                process_socket(cloned_state, socket).await;
            });
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

    if socket_url == "--tcp" {
        let socket_tcp_url: &str = match args.get(2) {
            Some(stringref) => stringref,
            None => DEFAULT_TCP_URL,
        };
        let tcp_socket = match TcpListener::bind(socket_tcp_url).await {
            Ok(x) => x,
            Err(x) => {
                println!("Error occurred while trying to bind: {}", x);
                return;
            }
        };
        loop_main!(tcp_socket, state);
    } else {
        let unix_socket = match UnixListener::bind(socket_url) {
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
        loop_main!(unix_socket, state);
    }
}

async fn process_socket<W>(state: Arc<GlobalState>, socket: W)
where
    W: AsyncReadExt + Send + AsyncWrite + 'static,
{
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
                    JobOpcode::PlayerUpdateTimestamp => {
                        let cloned_state = state.clone();
                        let cloned_sink = arc_sink.clone();
                        tokio::spawn(async move {
                            process_player_update_timestamp(
                                cloned_state,
                                cloned_sink,
                                opcode.request_id,
                            )
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
