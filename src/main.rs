mod consts;
mod jobs;
mod opcode;
mod player;

use ::futures::StreamExt;
use consts::{DEFAULT_SOCK_PATH, DEFAULT_TCP_URL};
use env_logger::Env;
use jobs::{process_decrypt_n_signature, process_fetch_update, GlobalState, JobOpcode};
use log::{debug, error, info};
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
        info!("Fetching player");
        match fetch_update($s.clone()).await {
            Ok(()) => info!("Successfully fetched player"),
            Err(x) => {
                error!("Error occurred while trying to fetch the player: {:?}", x);
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
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    let args: Vec<String> = args().collect();
    let socket_url: &str = args.get(1).map(String::as_ref).unwrap_or(DEFAULT_SOCK_PATH);

    // have to please rust
    let state: Arc<GlobalState> = Arc::new(GlobalState::new());

    if socket_url == "--tcp" {
        let socket_tcp_url = args.get(2).map(String::as_ref).unwrap_or(DEFAULT_TCP_URL);

        let tcp_socket = match TcpListener::bind(socket_tcp_url).await {
            Ok(x) => x,
            Err(x) => {
                error!("Error occurred while trying to bind: {}", x);
                return;
            }
        };

        loop_main!(tcp_socket, state);
    } else if socket_url == "--test" {
        // TODO: test the API as well, this only tests the player script extractor
        info!("Fetching player");

        std::process::exit(match fetch_update(state.clone()).await {
            Ok(_) => 0,
            Err(_) => -1,
        });
    } else {
        let unix_socket = match UnixListener::bind(socket_url) {
            Ok(x) => x,
            Err(x) if x.kind() == std::io::ErrorKind::AddrInUse => {
                let _ = remove_file(socket_url).await;
                UnixListener::bind(socket_url).unwrap()
            }
            Err(x) => {
                error!("Error occurred while trying to bind: {}", x);
                return;
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
                debug!("Received job: {}", opcode.opcode);

                let cloned_state = state.clone();
                let cloned_sink = arc_sink.clone();

                match opcode.opcode {
                    JobOpcode::ForceUpdate => {
                        tokio::spawn(async move {
                            process_fetch_update(cloned_state, cloned_sink, opcode.request_id)
                                .await;
                        });
                    }
                    JobOpcode::DecryptNSignature => {
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
                        tokio::spawn(async move {
                            process_player_status(cloned_state, cloned_sink, opcode.request_id)
                                .await;
                        });
                    }
                    JobOpcode::PlayerUpdateTimestamp => {
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
                error!("I/O error: {:?}", x);
                break;
            }
        }
    }
}
