use lazy_regex::{regex, Lazy};
use regex::Regex;

pub static DEFAULT_SOCK_PATH: &str = "/tmp/inv_sig_helper.sock";
pub static DEFAULT_SOCK_PERMS: u32 = 0o755;
pub static DEFAULT_TCP_URL: &str = "127.0.0.1:12999";

pub static TEST_YOUTUBE_VIDEO_ID: &str = "jNQXAC9IVRw";
pub static TEST_YOUTUBE_VIDEO: &str = concat!("https://www.youtube.com/watch?v=", TEST_YOUTUBE_VIDEO_ID);

pub static REGEX_PLAYER_ID: &Lazy<Regex> = regex!("\\/s\\/player\\/([0-9a-f]{8})");
pub static NSIG_FUNCTION_ARRAYS: &[&str] = &[
    r#"null\)&&\([a-zA-Z]=(?P<nfunc>[_a-zA-Z0-9$]+)\[(?P<idx>\d+)\]\([a-zA-Z0-9]\)"#,
    r#"(?x)&&\(b="n+"\[[a-zA-Z0-9.+$]+\],c=a\.get\(b\)\)&&\(c=(?P<nfunc>[a-zA-Z0-9$]+)(?:\[(?P<idx>\d+)\])?\([a-zA-Z0-9]\)"#,
];

pub static NSIG_FUNCTION_ENDINGS: &[&str] = &[
    r#"=\s*function(\(\w\)\s*\{[\S\s]*\{return.[a-zA-Z0-9_-]+_w8_.+?\}\s*return\s*\w+.join\(""\)\};)"#,
    r#"=\s*function([\S\s]*?\}\s*return \w+?\.join\(\"\"\)\s*\};)"#,
    r#"=\s*function([\S\s]*?\}\s*return [\W\w$]+?\.call\([\w$]+?,\"\"\)\s*\};)"#,
];

pub static REGEX_SIGNATURE_TIMESTAMP: &Lazy<Regex> = regex!("signatureTimestamp[=:](\\d+)");

pub static REGEX_SIGNATURE_FUNCTION: &Lazy<Regex> =
    regex!(r#"\s*?([a-zA-Z0-9_\$]{1,})=function\([a-zA-Z]{1}\)\{(.{1}=.{1}\.split\(""\)[^\}{]+)return .{1}\.join\(""\)\}"#);
pub static REGEX_HELPER_OBJ_NAME: &Lazy<Regex> = regex!(";([A-Za-z0-9_\\$]{2,})\\...\\(");

pub static NSIG_FUNCTION_NAME: &str = "decrypt_nsig";
pub static SIG_FUNCTION_NAME: &str = "decrypt_sig";

pub static ENV_USE_YT_DLP: &str = "USE_YT_DLP";