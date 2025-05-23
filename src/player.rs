use std::{sync::Arc, time::SystemTime};
use log::{debug, error, info, warn};
use regex::Regex;

use crate::{
    consts::{
        NSIG_FUNCTION_ARRAYS, NSIG_FUNCTION_ENDINGS, NSIG_FUNCTION_NAME, REGEX_HELPER_OBJ_NAME,
        REGEX_PLAYER_ID, REGEX_SIGNATURE_FUNCTION_PATTERNS, REGEX_SIGNATURE_TIMESTAMP, TEST_YOUTUBE_VIDEO,
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

fn extract_player_js_global_var(jscode: &str) -> Option<(String, String, String)> {
    let re = Regex::new(r#"(?x)
        'use\s+strict';\s*
        (?P<code>
            var\s+(?P<name>[a-zA-Z0-9_$]+)\s*=\s*
            (?P<value>
                (?:"[^"\\]*(?:\\.[^"\\]*)*"|'[^'\\]*(?:\\.[^'\\]*)*')
                \.split\((?:"[^"\\]*(?:\\.[^"\\]*)*"|'[^'\\]*(?:\\.[^'\\]*)*')\)
                |\[(?:(?:"[^"\\]*(?:\\.[^"\\]*)*"|'[^'\\]*(?:\\.[^'\\]*)*')\s*,?\s*)*\]
                |"[^"]*"\.split\("[^"]*"\)
            )
        )[;,]"#).ok()?;
    
    if let Some(caps) = re.captures(jscode) {
        Some((
            caps.name("code")?.as_str().to_string(),
            caps.name("name")?.as_str().to_string(),
            caps.name("value")?.as_str().to_string()
        ))
    } else {
        None
    }
}

fn fixup_nsig_jscode(jscode: &str, player_javascript: &str) -> String {
    // First try to extract any global variable
    let mut result = jscode.to_string();
    
    // Extract the original parameter name from the input JavaScript code
    let param_regex = Regex::new(r"function\s+[a-zA-Z0-9_$]+\s*\(([a-zA-Z0-9_$]+)\)").unwrap();
    let param_name = param_regex.captures(jscode)
        .and_then(|caps| caps.get(1))
        .map(|m| m.as_str())
        .unwrap_or("a"); // fallback to 'a' if we can't find the original parameter
    
    let fixup_re = if let Some((global_var, varname, _)) = extract_player_js_global_var(player_javascript) {
        debug!("global_var: {}", global_var);
        debug!("varname: {}", varname);
        debug!("jscode: {}", jscode);

        info!("Prepending n function code with global array variable '{}'", varname);
        result = format!("function decrypt_nsig({}){{{}; {}", param_name, global_var, jscode.replace(&format!("function decrypt_nsig({}){{", param_name), ""));

        // Escape special regex characters in the variable name
        let escaped_varname = varname.replace("$", "\\$");
        Regex::new(&format!(r#";\s*if\s*\(\s*typeof\s+[a-zA-Z0-9_$]+\s*===?\s*(?:"undefined"|'undefined'|{}\[\d+\])\s*\)\s*return\s+\w+;"#, escaped_varname)).unwrap()
    } else {
        info!("No global array variable found in player JS");
        Regex::new(r#";\s*if\s*\(\s*typeof\s+[a-zA-Z0-9_$]+\s*===?\s*"undefined"\s*\)\s*return\s+\w+;"#).unwrap()
    };

    // Now handle the conditional return statement cleanup
    // info!("jscode: {}", result);

    // Replace the matched pattern with just ";"
    if fixup_re.is_match(&result) {
        info!("Fixing up nsig_func_body");
        result = fixup_re.replace_all(&result, ";").to_string();
        // info!("result: {}", result);
    } else {
        info!("nsig_func returned with no fixup");
    }
    result
}

pub async fn fetch_update(state: Arc<GlobalState>) -> Result<(), FetchUpdateStatus> {
    let global_state = state.clone();
    let response = match reqwest::get(TEST_YOUTUBE_VIDEO).await {
        Ok(req) => req.text().await.unwrap(),
        Err(x) => {
            error!("Could not fetch the test video: {}", x);
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

    if player_id == current_player_id {
        current_player_info.last_update = SystemTime::now();
        return Err(FetchUpdateStatus::PlayerAlreadyUpdated);
    }
    // release the mutex for other tasks
    drop(current_player_info);
    
    // Download the player script
    let player_js_url: String = format!(
        "https://www.youtube.com/s/player/{:08x}/player_ias.vflset/en_US/base.js",
        player_id
    );
    info!("Fetching player JS URL: {}", player_js_url);
    let player_javascript = match reqwest::get(player_js_url).await {
        Ok(req) => req.text().await.unwrap(),
        Err(x) => {
            error!("Could not fetch the player JS: {}", x);
            return Err(FetchUpdateStatus::CannotFetchPlayerJS);
        }
    };

    let mut nsig_function_array_opt = None;
    // Extract nsig function array code
    for (index, nsig_function_array_str) in NSIG_FUNCTION_ARRAYS.iter().enumerate() {
        let nsig_function_array_regex = Regex::new(&nsig_function_array_str).unwrap();
        nsig_function_array_opt = match nsig_function_array_regex.captures(&player_javascript) {
            None => {
                warn!("nsig function array did not work: {}", nsig_function_array_str);
                if index == NSIG_FUNCTION_ARRAYS.len() {
                    error!("!!ERROR!! nsig function array unable to be extracted");
                    return Err(FetchUpdateStatus::NsigRegexCompileFailed);
                }
                continue;
            }
            Some(i) => {
                Some(i)
            }
        };
        break;
    }

    let nsig_function_array = nsig_function_array_opt.unwrap();
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
            error!("Error: nsig regex compilation failed: {}", x);
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

    let mut nsig_function_code = String::new();
    nsig_function_code += "function ";
    nsig_function_code += NSIG_FUNCTION_NAME;

    debug!("nsig function name: {}", nsig_function_name);

    // Extract nsig function code
    for (index, ending) in NSIG_FUNCTION_ENDINGS.iter().enumerate() {
        let mut nsig_function_code_regex_str: String = String::new();
        nsig_function_code_regex_str += "(?ms)";
        nsig_function_code_regex_str += &nsig_function_name.replace("$", "\\$");
        nsig_function_code_regex_str += ending;

        let nsig_function_code_regex = Regex::new(&nsig_function_code_regex_str).unwrap();
        nsig_function_code += match nsig_function_code_regex.captures(&player_javascript) {
            None => {
                warn!("nsig function ending did not work: {}", ending);
                if index == NSIG_FUNCTION_ENDINGS.len() {
                    error!("!!ERROR!! nsig function unable to be extracted");
                    return Err(FetchUpdateStatus::NsigRegexCompileFailed);
                }

                continue;
            }
            Some(i) => {
                debug!("nsig function ending worked: {}", ending);
                i.get(1).unwrap().as_str()
            }
        };
        nsig_function_code = fixup_nsig_jscode(&nsig_function_code, &player_javascript);
        debug!("got nsig fn code: {}", nsig_function_code);
        break;
    }

    let (global_var, varname, _) = extract_player_js_global_var(&player_javascript).unwrap();
    if !global_var.is_empty() {
        debug!("Found global var for sig: {}", global_var);
        debug!("Found varname for sig: {}", varname);
    } else {
        debug!("No global var found for sig");
    }

    // Extract signature function name
    let mut sig_function_name = String::new();
    let mut found_sig_function = false;
    
    for sig_pattern in REGEX_SIGNATURE_FUNCTION_PATTERNS.iter() {
        let _sig_pattern = sig_pattern.replace("GLOBAL_VAR_NAME", &regex::escape(&varname));

        debug!("sig pattern: {}", _sig_pattern);

        let sig_regex = match Regex::new(&_sig_pattern) {
            Ok(r) => r,
            Err(e) => {
                warn!("Failed to compile signature regex pattern: {}", e);
                continue;
            }
        };

        if let Some(cap) = sig_regex.captures(&player_javascript) {
            if let Some(m) = cap.get(1) {
                sig_function_name = m.as_str().to_string();
                found_sig_function = true;
                break;
            }
        }
    }

    if !found_sig_function {
        sig_function_name = format!("sig_function_{}", SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_millis());
        info!("No signature function found in player JS, using random name: {}", sig_function_name);
    }

    let mut sig_code = String::new();

    // if sig_function_name is not empty, then we need to extract the function body
    if found_sig_function {
        debug!("found sig function: {}", sig_function_name);
        
        let mut sig_function_body_regex_str: String = String::new();
        sig_function_body_regex_str += &sig_function_name.replace("$", "\\$");
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
        helper_object_body_regex_str += &helper_object_name.replace("$", "\\$");
        helper_object_body_regex_str += "=\\{(?:.|\\n)+?\\}\\};)";

        let helper_object_body_regex = Regex::new(&helper_object_body_regex_str).unwrap();
        let helper_object_body = helper_object_body_regex
            .captures(&player_javascript)
            .unwrap()
            .get(0)
            .unwrap()
            .as_str();

        sig_code += "var ";
        sig_code += &sig_function_name;
        sig_code += ";";

        if !global_var.is_empty() {
            sig_code += &global_var;
            sig_code += ";";
        }

        sig_code += helper_object_body;
        sig_code += sig_function_body;
    } else {
        // just return empty sig function code with random name
        sig_code += "var ";
        sig_code += &sig_function_name;
        sig_code += ";";
        info!("No signature function found in player JS, just returning empty sig function code with random name: {}", sig_function_name);
    }

    debug!("sig code: {}", sig_code);

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
    current_player_info.last_update = SystemTime::now();

    Ok(())
}
