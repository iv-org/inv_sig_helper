use futures::SinkExt;
use log::{debug, error};
use rquickjs::{async_with, AsyncContext, AsyncRuntime};
use std::{num::NonZeroUsize, sync::Arc, thread::available_parallelism, time::SystemTime};
use strum_macros::{Display, FromRepr};
use tokio::{runtime::Handle, sync::Mutex, task::block_in_place};
use tub::Pool;

use crate::{consts::NSIG_FUNCTION_NAME, opcode::OpcodeResponse, player::fetch_update};

#[derive(Display, FromRepr)]
pub enum JobOpcode {
    ForceUpdate = 0,
    DecryptNSignature = 1,
    DecryptSignature,
    GetSignatureTimestamp,
    PlayerStatus,
    PlayerUpdateTimestamp,
    UnknownOpcode = 255,
}

impl From<u8> for JobOpcode {
    fn from(value: u8) -> Self {
        JobOpcode::from_repr(value as usize).unwrap_or(Self::UnknownOpcode)
    }
}

pub struct PlayerInfo {
    pub nsig_function_code: String,
    pub sig_function_code: String,
    pub sig_function_name: String,
    pub signature_timestamp: u64,
    pub player_id: u32,
    pub has_player: u8,
    pub last_update: SystemTime,
}

impl Default for PlayerInfo {
    fn default() -> Self {
        Self {
            nsig_function_code: Default::default(),
            sig_function_code: Default::default(),
            sig_function_name: Default::default(),
            signature_timestamp: Default::default(),
            player_id: Default::default(),
            has_player: Default::default(),
            last_update: SystemTime::now(),
        }
    }
}

pub struct JavascriptInterpreter {
    #[allow(dead_code)]
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

        for _ in 0..number_of_runtimes {
            runtime_vector.push(Arc::new(JavascriptInterpreter::new()));
        }

        let js_runtime_pool: Pool<Arc<JavascriptInterpreter>> = Pool::from_vec(runtime_vector);

        GlobalState {
            player_info: Mutex::new(PlayerInfo::default()),
            js_runtime_pool,
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
    let _ = writer
        .send(OpcodeResponse {
            opcode: JobOpcode::ForceUpdate,
            request_id,
            update_status: status,
            ..Default::default()
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

    //println!("Signature to be decrypted: {}", sig);
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
                        error!("JavaScript interpreter error (nsig code): {:?}", ctx.catch().as_exception());
                    } else {
                        error!("JavaScript interpreter error (nsig code): {}", n);
                    }
                    debug!("Code: {}", player_info.nsig_function_code.clone());
                    writer = cloned_writer.lock().await;
                    let _ = writer.send(OpcodeResponse {
                        opcode: JobOpcode::DecryptNSignature,
                        request_id,
                        ..Default::default()
                    }).await;
                    return;
                }
            }
            *current_player_id = player_info.player_id;
        }
        drop(player_info);

        let call_string = format!("{NSIG_FUNCTION_NAME}(\"{}\")", sig.replace("\"", "\\\""));

        let decrypted_string = match ctx.eval::<String,String>(call_string.clone()) {
            Ok(x) => x,
            Err(n) => {
                if n.is_exception() {
                    error!("JavaScript interpreter error (nsig code): {:?}", ctx.catch().as_exception());
                } else {
                    error!("JavaScript interpreter error (nsig code): {}", n);
                }
                debug!("Code: {}", call_string.clone());
                writer = cloned_writer.lock().await;
                let _ = writer.send(OpcodeResponse {
                    opcode: JobOpcode::DecryptNSignature,
                    request_id,
                    ..Default::default()
                }).await;
                return;
            }
        };

        writer = cloned_writer.lock().await;

        let _ = writer.send(OpcodeResponse {
            opcode: JobOpcode::DecryptNSignature,
            request_id,
            signature: decrypted_string,
            ..Default::default()
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
                        error!("JavaScript interpreter error (sig code): {:?}", ctx.catch().as_exception());
                    } else {
                        error!("JavaScript interpreter error (sig code): {}", n);
                    }
                    debug!("Code: {}", player_info.sig_function_code.clone());
                    writer = cloned_writer.lock().await;
                    let _ = writer.send(OpcodeResponse {
                        opcode: JobOpcode::DecryptSignature,
                        request_id,
                        ..Default::default()
                    }).await;
                    return;
                }
            }
            *current_player_id = player_info.player_id;
        }

        let sig_function_name = &player_info.sig_function_name;

        let call_string = format!("{sig_function_name}(\"{}\")", sig.replace("\"", "\\\""));

        drop(player_info);

        let decrypted_string = match ctx.eval::<String,String>(call_string.clone()) {
            Ok(x) => x,
            Err(n) => {
                if n.is_exception() {
                    error!("JavaScript interpreter error (sig code): {:?}", ctx.catch().as_exception());
                } else {
                    error!("JavaScript interpreter error (sig code): {}", n);
                }
                debug!("Code: {}", call_string.clone());
                writer = cloned_writer.lock().await;
                let _ = writer.send(OpcodeResponse {
                    opcode: JobOpcode::DecryptSignature,
                    request_id,
                    ..Default::default()
                }).await;
                return;
            }
        };

        writer = cloned_writer.lock().await;

        let _ = writer.send(OpcodeResponse {
            opcode: JobOpcode::DecryptSignature,
            request_id,
            signature: decrypted_string,
            ..Default::default()
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
    let _ = writer
        .send(OpcodeResponse {
            opcode: JobOpcode::GetSignatureTimestamp,
            request_id,
            signature_timestamp: timestamp,
            ..Default::default()
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

    let _ = writer
        .send(OpcodeResponse {
            opcode: JobOpcode::PlayerStatus,
            request_id,
            has_player,
            player_id,
            ..Default::default()
        })
        .await;
}

pub async fn process_player_update_timestamp<W>(
    state: Arc<GlobalState>,
    stream: Arc<Mutex<W>>,
    request_id: u32,
) where
    W: SinkExt<OpcodeResponse> + Unpin + Send,
{
    let cloned_writer = stream.clone();
    let global_state = state.clone();

    let player_info = global_state.player_info.lock().await;
    let last_update = player_info.last_update;

    let mut writer = cloned_writer.lock().await;

    let _ = writer
        .send(OpcodeResponse {
            opcode: JobOpcode::PlayerUpdateTimestamp,
            request_id,
            last_player_update: SystemTime::now()
                .duration_since(last_update)
                .unwrap()
                .as_secs(),

            ..Default::default()
        })
        .await;
}
