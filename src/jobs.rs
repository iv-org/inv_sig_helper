use regex::Regex;
use rquickjs::{AsyncContext, AsyncRuntime};
use std::{num::NonZeroUsize, ops::Deref, sync::Arc, thread::available_parallelism};
use tokio::{runtime::Handle, sync::Mutex, task::block_in_place};
use tub::Pool;

use crate::consts::{NSIG_FUNCTION_ARRAY, REGEX_PLAYER_ID, TEST_YOUTUBE_VIDEO};

pub enum JobOpcode {
    ForceUpdate,
    DecryptNSignature,
    UnknownOpcode,
}

impl From<u8> for JobOpcode {
    fn from(value: u8) -> Self {
        match value {
            0x00 => Self::ForceUpdate,
            0x01 => Self::DecryptNSignature,

            // make debugging easier
            b'a' => Self::ForceUpdate,
            _ => Self::UnknownOpcode,
        }
    }
}

pub struct PlayerInfo {
    nsig_function_code: String,
    player_id: u32,
}

pub struct JavascriptInterpreter {
    js_runtime: AsyncRuntimeWrapper,
    nsig_context: AsyncContextWrapper,
    player_id: u32,
}

// This is to get Rust to shut up, since the types are aliases for non-null pointers
struct AsyncRuntimeWrapper(AsyncRuntime);
struct AsyncContextWrapper(AsyncContext);

unsafe impl Send for AsyncRuntimeWrapper {}
unsafe impl Send for AsyncContextWrapper {}
unsafe impl Sync for AsyncRuntimeWrapper {}
unsafe impl Sync for AsyncContextWrapper {}
impl Deref for AsyncRuntimeWrapper {
    type Target = AsyncRuntime;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Deref for AsyncContextWrapper {
    type Target = AsyncContext;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
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
        JavascriptInterpreter {
            js_runtime: AsyncRuntimeWrapper(js_runtime),
            nsig_context: AsyncContextWrapper(nsig_context),
            player_id: 0,
        }
    }
}

pub struct GlobalState {
    player_info: Mutex<PlayerInfo>,
    js_runtime_pool: Mutex<Pool<Arc<JavascriptInterpreter>>>,
    all_runtimes: Mutex<Vec<Arc<JavascriptInterpreter>>>,
}

impl GlobalState {
    pub fn new() -> GlobalState {
        let number_of_runtimes = available_parallelism()
            .unwrap_or(NonZeroUsize::new(1).unwrap())
            .get();
        let mut runtime_vector: Vec<Arc<JavascriptInterpreter>> =
            Vec::with_capacity(number_of_runtimes);
        for n in 0..number_of_runtimes {
            runtime_vector.push(Arc::new(JavascriptInterpreter::new()));
        }
        // Make a clone of the vector, this will clone all the values inside (Arc)
        let mut runtime_vector2: Vec<Arc<JavascriptInterpreter>> =
            Vec::with_capacity(number_of_runtimes);
        runtime_vector2.clone_from(&runtime_vector);
        let runtime_pool: Pool<Arc<JavascriptInterpreter>> = Pool::from_vec(runtime_vector2);
        GlobalState {
            player_info: Mutex::new(PlayerInfo {
                nsig_function_code: Default::default(),
                player_id: Default::default(),
            }),
            js_runtime_pool: Mutex::new(runtime_pool),
            all_runtimes: Mutex::new(runtime_vector),
        }
    }
}
pub async fn process_fetch_update(state: Arc<GlobalState>) {
    let global_state = state.clone();
    let response = match reqwest::get(TEST_YOUTUBE_VIDEO).await {
        Ok(req) => req.text().await.unwrap(),
        Err(x) => {
            println!("Could not fetch the test video: {}", x);
            return;
        }
    };

    let player_id_str = match REGEX_PLAYER_ID.captures(&response).unwrap().get(1) {
        Some(result) => result.as_str(),
        None => return,
    };

    let player_id: u32 = u32::from_str_radix(player_id_str, 16).unwrap();

    let mut current_player_info = global_state.player_info.lock().await;
    let current_player_id = current_player_info.player_id;
    // release the mutex for other tasks
    drop(current_player_info);

    if player_id == current_player_id {
        // Player is already up to date
        return;
    }

    // Download the player script
    let player_js_url: String = format!(
        "https://www.youtube.com/s/player/{:08x}/player_ias.vflset/en_US/base.js",
        player_id
    );
    let player_javascript = match reqwest::get(player_js_url).await {
        Ok(req) => req.text().await.unwrap(),
        Err(x) => {
            println!("Could not fetch the player JS: {}", x);
            return;
        }
    };

    let nsig_function_array = NSIG_FUNCTION_ARRAY.captures(&player_javascript).unwrap();
    let nsig_array_name = nsig_function_array.get(1).unwrap().as_str();
    let nsig_array_value = nsig_function_array
        .get(2)
        .unwrap()
        .as_str()
        .parse::<usize>()
        .unwrap();

    let mut nsig_array_context_regex: String = String::new();
    nsig_array_context_regex += "var ";
    nsig_array_context_regex += nsig_array_name;
    nsig_array_context_regex += "\\s*=\\s*\\[(.+?)][;,]";

    let nsig_array_context = match Regex::new(&nsig_array_context_regex) {
        Ok(x) => x,
        Err(x) => {
            println!("Error: nsig regex compilation failed: {}", x);
            return;
        }
    };

    let array_content = nsig_array_context
        .captures(&player_javascript)
        .unwrap()
        .get(1)
        .unwrap()
        .as_str()
        .split(",");

    let array_values: Vec<&str> = array_content.collect();

    let nsig_function_name = array_values.get(nsig_array_value).unwrap();

    // Extract nsig function code
    let mut nsig_function_code_regex_str: String = String::new();
    nsig_function_code_regex_str += nsig_function_name;
    nsig_function_code_regex_str +=
        "=\\s*function([\\S\\s]*?\\}\\s*return [\\w$]+?\\.join\\(\"\"\\)\\s*\\};)";

    let nsig_function_code_regex = Regex::new(&nsig_function_code_regex_str).unwrap();

    let mut nsig_function_code = String::new();
    nsig_function_code += "decrypt_nsig = function";
    nsig_function_code += nsig_function_code_regex
        .captures(&player_javascript)
        .unwrap()
        .get(1)
        .unwrap()
        .as_str();

    current_player_info = global_state.player_info.lock().await;
    current_player_info.player_id = player_id;
    current_player_info.nsig_function_code = nsig_function_code;
    println!("{}", current_player_info.nsig_function_code);
}

pub async fn process_decrypt_n_signature(_state: Arc<GlobalState>, _sig: String) {}
