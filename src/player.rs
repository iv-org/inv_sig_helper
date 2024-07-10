use std::sync::Arc;

use regex::Regex;

use crate::{
    consts::{
        NSIG_FUNCTION_ARRAY, NSIG_FUNCTION_NAME, REGEX_HELPER_OBJ_NAME, REGEX_PLAYER_ID,
        REGEX_SIGNATURE_FUNCTION, REGEX_SIGNATURE_TIMESTAMP, TEST_YOUTUBE_VIDEO,
    },
    jobs::GlobalState,
};

// TODO: too lazy to make proper debugging print
#[derive(Debug)]
pub enum FetchUpdateStatus {
    CannotFetchTestVideo,
    CannotMatchPlayerID,
    CannotFetchPlayerJS,
    NsigRegexCompileFailed,
    PlayerAlreadyUpdated,
}

pub async fn fetch_update(state: Arc<GlobalState>) -> Result<(), FetchUpdateStatus> {
    let global_state = state.clone();
    let response = match reqwest::get(TEST_YOUTUBE_VIDEO).await {
        Ok(req) => req.text().await.unwrap(),
        Err(x) => {
            println!("Could not fetch the test video: {}", x);
            return Err(FetchUpdateStatus::CannotFetchTestVideo);
        }
    };

    let player_id_str = match REGEX_PLAYER_ID.captures(&response).unwrap().get(1) {
        Some(result) => result.as_str(),
        None => return Err(FetchUpdateStatus::CannotMatchPlayerID),
    };

    let player_id: u32 = u32::from_str_radix(player_id_str, 16).unwrap();

    let mut current_player_info = global_state.player_info.lock().await;
    let current_player_id = current_player_info.player_id;
    // release the mutex for other tasks
    drop(current_player_info);

    if player_id == current_player_id {
        return Err(FetchUpdateStatus::PlayerAlreadyUpdated);
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
            return Err(FetchUpdateStatus::CannotFetchPlayerJS);
        }
    };

    let nsig_function_array = NSIG_FUNCTION_ARRAY.captures(&player_javascript).unwrap();
    let nsig_array_name = nsig_function_array.name("nfunc").unwrap().as_str();
    let nsig_array_value = nsig_function_array
        .name("idx")
        .unwrap()
        .as_str()
        .parse::<usize>()
        .unwrap();

    let mut nsig_array_context_regex: String = String::new();
    nsig_array_context_regex += "var ";
    nsig_array_context_regex += &nsig_array_name.replace("$", "\\$");
    nsig_array_context_regex += "\\s*=\\s*\\[(.+?)][;,]";

    let nsig_array_context = match Regex::new(&nsig_array_context_regex) {
        Ok(x) => x,
        Err(x) => {
            println!("Error: nsig regex compilation failed: {}", x);
            return Err(FetchUpdateStatus::NsigRegexCompileFailed);
        }
    };

    let array_content = nsig_array_context
        .captures(&player_javascript)
        .unwrap()
        .get(1)
        .unwrap()
        .as_str()
        .split(',');

    let array_values: Vec<&str> = array_content.collect();

    let nsig_function_name = array_values.get(nsig_array_value).unwrap();

    // Extract nsig function code
    let mut nsig_function_code_regex_str: String = String::new();
    nsig_function_code_regex_str += &nsig_function_name.replace("$", "\\$");
    nsig_function_code_regex_str +=
        "=\\s*function([\\S\\s]*?\\}\\s*return [\\W\\w$]+?\\.call\\([\\w$]+?,\"\"\\)\\s*\\};)";

    let nsig_function_code_regex = Regex::new(&nsig_function_code_regex_str).unwrap();

    let mut nsig_function_code = String::new();
    nsig_function_code += "function ";
    nsig_function_code += NSIG_FUNCTION_NAME;
    nsig_function_code += nsig_function_code_regex
        .captures(&player_javascript)
        .unwrap()
        .get(1)
        .unwrap()
        .as_str();

    // Extract signature function name
    let sig_function_name = REGEX_SIGNATURE_FUNCTION
        .captures(&player_javascript)
        .unwrap()
        .get(1)
        .unwrap()
        .as_str();

    let mut sig_function_body_regex_str: String = String::new();
    sig_function_body_regex_str += sig_function_name;
    sig_function_body_regex_str += "=function\\([a-zA-Z0-9_]+\\)\\{.+?\\}";

    let sig_function_body_regex = Regex::new(&sig_function_body_regex_str).unwrap();

    let sig_function_body = sig_function_body_regex
        .captures(&player_javascript)
        .unwrap()
        .get(0)
        .unwrap()
        .as_str();

    // Get the helper object
    let helper_object_name = REGEX_HELPER_OBJ_NAME
        .captures(sig_function_body)
        .unwrap()
        .get(1)
        .unwrap()
        .as_str();

    let mut helper_object_body_regex_str = String::new();
    helper_object_body_regex_str += "(var ";
    helper_object_body_regex_str += helper_object_name;
    helper_object_body_regex_str += "=\\{(?:.|\\n)+?\\}\\};)";

    let helper_object_body_regex = Regex::new(&helper_object_body_regex_str).unwrap();
    let helper_object_body = helper_object_body_regex
        .captures(&player_javascript)
        .unwrap()
        .get(0)
        .unwrap()
        .as_str();

    let mut sig_code = String::new();
    sig_code += "var ";
    sig_code += sig_function_name;
    sig_code += ";";

    sig_code += helper_object_body;
    sig_code += sig_function_body;

    println!("{}", sig_code);

    // Get signature timestamp
    let signature_timestamp: u64 = REGEX_SIGNATURE_TIMESTAMP
        .captures(&player_javascript)
        .unwrap()
        .get(1)
        .unwrap()
        .as_str()
        .parse()
        .unwrap();

    current_player_info = global_state.player_info.lock().await;
    current_player_info.player_id = player_id;
    current_player_info.nsig_function_code = nsig_function_code;
    current_player_info.sig_function_code = sig_code;
    current_player_info.sig_function_name = sig_function_name.to_string();
    current_player_info.signature_timestamp = signature_timestamp;
    current_player_info.has_player = 0xFF;

    Ok(())
}
