use futures::{Sink, SinkExt, Stream, StreamExt};
use rquickjs::{async_with, AsyncContext, AsyncRuntime};
use std::{num::NonZeroUsize, sync::Arc, thread::available_parallelism};
use tokio::{io::AsyncWriteExt, runtime::Handle, sync::Mutex, task::block_in_place};
use tub::Pool;

use crate::{
    consts::NSIG_FUNCTION_NAME,
    opcode::OpcodeResponse,
    player::{fetch_update, FetchUpdateStatus},
};

pub enum JobOpcode {
    ForceUpdate,
    DecryptNSignature,
    DecryptSignature,
    GetSignatureTimestamp,
    PlayerStatus,
    UnknownOpcode,
}

impl std::fmt::Display for JobOpcode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ForceUpdate => write!(f, "ForceUpdate"),
            Self::DecryptNSignature => write!(f, "DecryptNSignature"),
            Self::DecryptSignature => write!(f, "DecryptSignature"),
            Self::GetSignatureTimestamp => write!(f, "GetSignatureTimestamp"),
            Self::PlayerStatus => write!(f, "PlayerStatus"),
            Self::UnknownOpcode => write!(f, "UnknownOpcode"),
        }
    }
}
impl From<u8> for JobOpcode {
    fn from(value: u8) -> Self {
        match value {
            0x00 => Self::ForceUpdate,
            0x01 => Self::DecryptNSignature,
            0x02 => Self::DecryptSignature,
            0x03 => Self::GetSignatureTimestamp,
            0x04 => Self::PlayerStatus,
            _ => Self::UnknownOpcode,
        }
    }
}

pub struct PlayerInfo {
    pub nsig_function_code: String,
    pub sig_function_code: String,
    pub sig_function_name: String,
    pub signature_timestamp: u64,
    pub player_id: u32,
    pub has_player: u8,
}

pub struct JavascriptInterpreter {
    js_runtime: AsyncRuntime,
    sig_context: AsyncContext,
    nsig_context: AsyncContext,
    sig_player_id: Mutex<u32>,
    nsig_player_id: Mutex<u32>,
}

impl JavascriptInterpreter {
    pub fn new() -> JavascriptInterpreter {
        let js_runtime = AsyncRuntime::new().unwrap();
        // not ideal, but this is only done at startup
        let nsig_context = block_in_place(|| {
            Handle::current()
                .block_on(AsyncContext::full(&js_runtime))
                .unwrap()
        });
        let sig_context = block_in_place(|| {
            Handle::current()
                .block_on(AsyncContext::full(&js_runtime))
                .unwrap()
        });
        JavascriptInterpreter {
            js_runtime,
            sig_context,
            nsig_context,
            sig_player_id: Mutex::new(0),
            nsig_player_id: Mutex::new(0),
        }
    }
}

pub struct GlobalState {
    pub player_info: Mutex<PlayerInfo>,
    js_runtime_pool: Pool<Arc<JavascriptInterpreter>>,
}

impl GlobalState {
    pub fn new() -> GlobalState {
        let number_of_runtimes = available_parallelism()
            .unwrap_or(NonZeroUsize::new(1).unwrap())
            .get();
        let mut runtime_vector: Vec<Arc<JavascriptInterpreter>> =
            Vec::with_capacity(number_of_runtimes);
        for _n in 0..number_of_runtimes {
            runtime_vector.push(Arc::new(JavascriptInterpreter::new()));
        }

        let runtime_pool: Pool<Arc<JavascriptInterpreter>> = Pool::from_vec(runtime_vector);
        GlobalState {
            player_info: Mutex::new(PlayerInfo {
                nsig_function_code: Default::default(),
                sig_function_code: Default::default(),
                sig_function_name: Default::default(),
                player_id: Default::default(),
                signature_timestamp: Default::default(),
                has_player: 0x00,
            }),
            js_runtime_pool: runtime_pool,
        }
    }
}

pub async fn process_fetch_update<W>(
    state: Arc<GlobalState>,
    stream: Arc<Mutex<W>>,
    request_id: u32,
) where
    W: SinkExt<OpcodeResponse> + Unpin + Send,
{
    let cloned_writer = stream.clone();
    let global_state = state.clone();
    let status = fetch_update(global_state).await;

    let mut writer = cloned_writer.lock().await;
    writer
        .send(OpcodeResponse {
            opcode: JobOpcode::ForceUpdate,
            request_id,
            update_status: status,
            signature: Default::default(),
            signature_timestamp: Default::default(),
            has_player: Default::default(),
            player_id: Default::default(),
        })
        .await;
}

pub async fn process_decrypt_n_signature<W>(
    state: Arc<GlobalState>,
    sig: String,
    stream: Arc<Mutex<W>>,
    request_id: u32,
) where
    W: SinkExt<OpcodeResponse> + Unpin + Send,
{
    let cloned_writer = stream.clone();
    let global_state = state.clone();

    println!("Signature to be decrypted: {}", sig);
    let interp = global_state.js_runtime_pool.acquire().await;

    let cloned_interp = interp.clone();
    async_with!(cloned_interp.nsig_context => |ctx|{
        let mut writer;
        let mut current_player_id = interp.nsig_player_id.lock().await;
        let player_info = global_state.player_info.lock().await;

        if player_info.player_id != *current_player_id {
            match ctx.eval::<(),String>(player_info.nsig_function_code.clone()) {
                Ok(x) => x,
                Err(n) => {
                    if n.is_exception() {
                        println!("JavaScript interpreter error (nsig code): {:?}", ctx.catch().as_exception());
                    } else {
                        println!("JavaScript interpreter error (nsig code): {}", n);
                    }
                    println!("Code: {}", player_info.nsig_function_code.clone());
                    writer = cloned_writer.lock().await;
                    writer.send(OpcodeResponse {
                        opcode: JobOpcode::DecryptNSignature,
                        request_id,
                        update_status: Ok(Default::default()),
                        signature: String::new(),
                        signature_timestamp: Default::default(),
                        has_player: Default::default(),
                        player_id: Default::default(),
                    }).await;
                    return;
                }
            }
            *current_player_id = player_info.player_id;
        }
        drop(player_info);

        let mut call_string: String = String::new();
        call_string += NSIG_FUNCTION_NAME;
        call_string += "(\"";
        call_string += &sig.replace("\"", "\\\"");
        call_string += "\")";

        let decrypted_string = match ctx.eval::<String,String>(call_string.clone()) {
            Ok(x) => x,
            Err(n) => {
                if n.is_exception() {
                    println!("JavaScript interpreter error (nsig code): {:?}", ctx.catch().as_exception());
                } else {
                    println!("JavaScript interpreter error (nsig code): {}", n);
                }
                println!("Code: {}", call_string.clone());
                writer = cloned_writer.lock().await;
                writer.send(OpcodeResponse {
                    opcode: JobOpcode::DecryptNSignature,
                    request_id,
                    update_status: Ok(Default::default()),
                    signature: String::new(),
                    signature_timestamp: Default::default(),
                    has_player: Default::default(),
                    player_id: Default::default(),
                }).await;
                return;
            }
        };

        writer = cloned_writer.lock().await;

        writer.send(OpcodeResponse {
            opcode: JobOpcode::DecryptNSignature,
            request_id,
            update_status: Ok(Default::default()),
            signature: decrypted_string,
            signature_timestamp: Default::default(),
            has_player: Default::default(),
            player_id: Default::default(),
        }).await;
    })
    .await;
}

pub async fn process_decrypt_signature<W>(
    state: Arc<GlobalState>,
    sig: String,
    stream: Arc<Mutex<W>>,
    request_id: u32,
) where
    W: SinkExt<OpcodeResponse> + Unpin + Send,
{
    let cloned_writer = stream.clone();
    let global_state = state.clone();

    let interp = global_state.js_runtime_pool.acquire().await;
    let cloned_interp = interp.clone();

    async_with!(cloned_interp.sig_context => |ctx|{
        let mut writer;
        let mut current_player_id = interp.sig_player_id.lock().await;
        let player_info = global_state.player_info.lock().await;

        if player_info.player_id != *current_player_id {
            match ctx.eval::<(),String>(player_info.sig_function_code.clone()) {
                Ok(x) => x,
                Err(n) => {
                    if n.is_exception() {
                        println!("JavaScript interpreter error (sig code): {:?}", ctx.catch().as_exception());
                    } else {
                        println!("JavaScript interpreter error (sig code): {}", n);
                    }
                    println!("Code: {}", player_info.sig_function_code.clone());
                    writer = cloned_writer.lock().await;
                    writer.send(OpcodeResponse {
                        opcode: JobOpcode::DecryptSignature,
                        request_id,
                        update_status: Ok(Default::default()),
                        signature: String::new(),
                        signature_timestamp: Default::default(),
                        has_player: Default::default(),
                        player_id: Default::default(),
                    }).await;
                    return;
                }
            }
            *current_player_id = player_info.player_id;
        }

        let sig_function_name = &player_info.sig_function_name;

        let mut call_string: String = String::new();
        call_string += sig_function_name;
        call_string += "(\"";
        call_string += &sig.replace("\"", "\\\"");
        call_string += "\")";

        drop(player_info);

        let decrypted_string = match ctx.eval::<String,String>(call_string.clone()) {
            Ok(x) => x,
            Err(n) => {
                if n.is_exception() {
                    println!("JavaScript interpreter error (sig code): {:?}", ctx.catch().as_exception());
                } else {
                    println!("JavaScript interpreter error (sig code): {}", n);
                }
                println!("Code: {}", call_string.clone());
                writer = cloned_writer.lock().await;
                writer.send(OpcodeResponse {
                    opcode: JobOpcode::DecryptSignature,
                    request_id,
                    update_status: Ok(Default::default()),
                    signature: String::new(),
                    signature_timestamp: Default::default(),
                    has_player: Default::default(),
                    player_id: Default::default(),
                }).await;
                return;
            }
        };

        writer = cloned_writer.lock().await;

        writer.send(OpcodeResponse {
            opcode: JobOpcode::DecryptSignature,
            request_id,
            update_status: Ok(Default::default()),
            signature: decrypted_string,
            signature_timestamp: Default::default(),
            has_player: Default::default(),
            player_id: Default::default(),
        }).await;
    })
    .await;
}

pub async fn process_get_signature_timestamp<W>(
    state: Arc<GlobalState>,
    stream: Arc<Mutex<W>>,
    request_id: u32,
) where
    W: SinkExt<OpcodeResponse> + Unpin + Send,
{
    let cloned_writer = stream.clone();
    let global_state = state.clone();

    let player_info = global_state.player_info.lock().await;
    let timestamp = player_info.signature_timestamp;

    let mut writer = cloned_writer.lock().await;
    writer
        .send(OpcodeResponse {
            opcode: JobOpcode::GetSignatureTimestamp,
            request_id,
            update_status: Ok(Default::default()),
            signature: String::new(),
            signature_timestamp: timestamp,
            has_player: Default::default(),
            player_id: Default::default(),
        })
        .await;
}

pub async fn process_player_status<W>(
    state: Arc<GlobalState>,
    stream: Arc<Mutex<W>>,
    request_id: u32,
) where
    W: SinkExt<OpcodeResponse> + Unpin + Send,
{
    let cloned_writer = stream.clone();
    let global_state = state.clone();

    let player_info = global_state.player_info.lock().await;
    let has_player = player_info.has_player;
    let player_id = player_info.player_id;

    let mut writer = cloned_writer.lock().await;

    writer
        .send(OpcodeResponse {
            opcode: JobOpcode::PlayerStatus,
            request_id,
            update_status: Ok(Default::default()),
            signature: String::new(),
            signature_timestamp: Default::default(),
            has_player,
            player_id,
        })
        .await;
}
