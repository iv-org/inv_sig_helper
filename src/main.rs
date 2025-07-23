mod consts;
mod jobs;
mod opcode;
mod player;

use ::futures::StreamExt;
use consts::{DEFAULT_SOCK_PATH, DEFAULT_SOCK_PERMS, DEFAULT_TCP_URL};
use jobs::{process_decrypt_n_signature, process_fetch_update, GlobalState, JobOpcode};
use opcode::OpcodeDecoder;
use player::fetch_update;
use std::{env::args, sync::Arc, fs::set_permissions, fs::Permissions, os::unix::fs::PermissionsExt};
use env_logger::Env;
use tokio::{
    fs::remove_file,
    io::{AsyncReadExt, AsyncWrite},
    net::{TcpListener, UnixListener},
    sync::Mutex,
    signal,
    select,
};
use tokio_util::codec::Framed;
use log::{info, error, debug};

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
                error!("Error occured while trying to fetch the player: {:?}", x);
            }
        }
        let mut sigterm = signal::unix::signal(signal::unix::SignalKind::terminate()).unwrap();
        let mut sigint = signal::unix::signal(signal::unix::SignalKind::interrupt()).unwrap();

        loop {
            select! {
                result = $i.accept() => {
                    let (socket, _addr) = result.unwrap();
                    let cloned_state = $s.clone();
                    tokio::spawn(async move {
                        process_socket(cloned_state, socket).await;
                    });
                }
                _ = sigterm.recv() => break,
                _ = sigint.recv() => break,
            }
        }
    };
}
#[tokio::main]
async fn main() {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

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
                error!("Error occurred while trying to bind: {}", x);
                return;
            }
        };
        loop_main!(tcp_socket, state);
    } else if socket_url == "--test" {
        // TODO: test the API aswell, this only tests the player script extractor
        info!("Fetching player");
        match fetch_update(state.clone()).await {
            Ok(()) => std::process::exit(0),
            Err(_x) => std::process::exit(-1),
        }
    } else {
        let unix_socket = match UnixListener::bind(socket_url) {
            Ok(x) => x,
            Err(x) => {
                if x.kind() == std::io::ErrorKind::AddrInUse {
                    remove_file(socket_url).await;
                    UnixListener::bind(socket_url).unwrap()
                } else {
                    error!("Error occurred while trying to bind: {}", x);
                    return;
                }
            }
        };
        let socket_perms: u32 = match args.get(2) {
            Some(stringref) => u32::from_str_radix(stringref, 8).expect(
                "Socket permissions must be an octal from 0 to 777!"),
            None => DEFAULT_SOCK_PERMS,
        };
        let perms = Permissions::from_mode(socket_perms);
        let _ = set_permissions(socket_url, perms);
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
                error!("I/O error: {:?}", x);
                break;
            }
        }
    }
}
