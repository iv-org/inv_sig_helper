use std::env::var;
use std::path::PathBuf;
use std::process::Command::new;

use crate::consts::{ENV_USE_YT_DLP, TEST_YOUTUBE_VIDEO};

fn ytdlp_get_script_path(script_name: &str) -> PathBuf {
    let exe_path = std::env::current_exe().expect("Failed to get current path of binary");
    let exe_dir = exe_path.parent().expect("Failed to get current path of binary");
    exe_dir.join("scripts").join(script_name)
}

pub fn ytdlp_requested() -> bool {
    match std::env::var(ENV_USE_YT_DLP) {
        Ok(val) => val == "1",
        Err(_) => false,
    }
}

pub fn ytdlp_signature_timestamp(player_id: u32) -> u64 {
    let player_js_url: String = format!(
        "https://www.youtube.com/s/player/{:08x}/player_ias.vflset/en_US/base.js",
        player_id
    );
    let child = std::process::Command::new(ytdlp_get_script_path("ytdlp_signature_timestamp.py"))
        .arg(player_js_url)
        .arg(TEST_YOUTUBE_VIDEO)
        .output()
        .expect("Failed to execute command");

    let output = String::from_utf8_lossy(&child.stdout);
    output.to_string().parse::<u64>().unwrap()
}

pub fn ytdlp_nsig_decoder(signature: &str, player_id: u32) -> String {
    let player_js_url: String = format!(
        "https://www.youtube.com/s/player/{:08x}/player_ias.vflset/en_US/base.js",
        player_id
    );
    let child = std::process::Command::new(ytdlp_get_script_path("ytdlp_nsig_decoder.py"))
        .arg(player_js_url)
        .arg(signature)
        .arg(TEST_YOUTUBE_VIDEO)
        .output()
        .expect("Failed to execute command");

    let output = String::from_utf8_lossy(&child.stdout);
    output.to_string()
}

pub fn ytdlp_sig_decoder(signature: &str, player_id: u32) -> String {
    let player_js_url: String = format!(
        "https://www.youtube.com/s/player/{:08x}/player_ias.vflset/en_US/base.js",
        player_id
    );
    let child = std::process::Command::new(ytdlp_get_script_path("ytdlp_sig_decoder.py"))
        .arg(player_js_url)
        .arg(signature)
        .arg(TEST_YOUTUBE_VIDEO)
        .output()
        .expect("Failed to execute command");

    let output = String::from_utf8_lossy(&child.stdout);
    output.to_string()
}

