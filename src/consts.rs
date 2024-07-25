use lazy_regex::{regex, Lazy};
use regex::Regex;

pub static DEFAULT_SOCK_PATH: &str = "/tmp/inv_sig_helper.sock";
pub static DEFAULT_TCP_URL: &str = "127.0.0.1:12999";

pub static TEST_YOUTUBE_VIDEO: &str = "https://www.youtube.com/watch?v=jNQXAC9IVRw";

pub static REGEX_PLAYER_ID: &Lazy<Regex> = regex!("\\/s\\/player\\/([0-9a-f]{8})");
pub static NSIG_FUNCTION_ARRAY: &Lazy<Regex> = regex!(
    r#"(?x)&&\(b="n+"\[[a-zA-Z0-9.+$]+\],c=a\.get\(b\)\)&&\(c=(?P<nfunc>[a-zA-Z0-9$]+)(?:\[(?P<idx>\d+)\])?\([a-zA-Z0-9]\)"#
);

pub static NSIG_FUNCTION_ENDINGS: &[&str] = &[
    "=\\s*function([\\S\\s]*?\\}\\s*return \\w+?\\.join\\(\"\"\\)\\s*\\};)",
    "=\\s*function([\\S\\s]*?\\}\\s*return [\\W\\w$]+?\\.call\\([\\w$]+?,\"\"\\)\\s*\\};)",
];

pub static REGEX_SIGNATURE_TIMESTAMP: &Lazy<Regex> = regex!("signatureTimestamp[=:](\\d+)");

pub static REGEX_SIGNATURE_FUNCTION: &Lazy<Regex> =
    regex!("\\bc&&\\(c=([a-zA-Z0-9$]{2,})\\(decodeURIComponent\\(c\\)\\)");
pub static REGEX_HELPER_OBJ_NAME: &Lazy<Regex> = regex!(";([A-Za-z0-9_\\$]{2,})\\...\\(");

pub static NSIG_FUNCTION_NAME: &str = "decrypt_nsig";
pub static SIG_FUNCTION_NAME: &str = "decrypt_sig";
