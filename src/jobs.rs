use std::sync::Arc;
use tokio::sync::Mutex;

use crate::consts::{REGEX_PLAYER_ID, TEST_YOUTUBE_VIDEO};

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
            _ => Self::UnknownOpcode,
        }
    }
}

pub struct PlayerInfo {
    nsig_function_bytecode: Vec<u8>,
    player_id: u32,
}
pub struct GlobalState {
    player_info: Mutex<PlayerInfo>,
}

impl GlobalState {
    pub fn new() -> GlobalState {
        return GlobalState {
            player_info: Mutex::new(PlayerInfo {
                nsig_function_bytecode: Default::default(),
                player_id: Default::default(),
            }),
        };
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

    let player_id_str = match REGEX_PLAYER_ID.captures(&response).unwrap().get(0) {
        Some(result) => result.as_str(),
        None => return,
    };

    let player_id: u32 = u32::from_str_radix(player_id_str, 16).unwrap();

    let current_player_info = global_state.player_info.lock().await;
    let current_player_id = current_player_info.player_id;
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
}
pub async fn process_decrypt_n_signature(state: Arc<GlobalState>, sig: String) {}
