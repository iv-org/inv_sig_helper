use lazy_regex::{regex, Lazy};
use regex::Regex;

pub static DEFAULT_SOCK_PATH: &str = "/tmp/inv_sig_helper.sock";
pub static DEFAULT_TCP_URL: &str = "127.0.0.1:12999";

pub static TEST_YOUTUBE_VIDEO: &str = "https://www.youtube.com/watch?v=jNQXAC9IVRw";

pub static REGEX_PLAYER_ID: &Lazy<Regex> = regex!("\\/s\\/player\\/([0-9a-f]{8})");
pub static NSIG_FUNCTION_ARRAYS: &[&str] = &[
    r#"null\)&&\([a-zA-Z]=(?P<nfunc>[a-zA-Z0-9$]+)\[(?P<idx>\d+)\]\([a-zA-Z0-9]\)"#,
    r#"(?x)&&\(b="n+"\[[a-zA-Z0-9.+$]+\],c=a\.get\(b\)\)&&\(c=(?P<nfunc>[a-zA-Z0-9$]+)(?:\[(?P<idx>\d+)\])?\([a-zA-Z0-9]\)"#,
];

pub static NSIG_FUNCTION_ENDINGS: &[&str] = &[
    r#"=\s*function(\([\w]+\)\{\s*var\s+[\w\s]+=[\w\.\s]+?\.call\s*\([\w\s$]+?,[\(\)\",\s]+\)[\S\s]*?\}\s*return [\w\.\s$]+?\.call\s*\([\w\s$]+?\s*,[\(\)\",\s]+\)\s*\}\s*;)"#,
    r#"=\s*function([\S\s]*?\}\s*return \w+?\.join\(\"\"\)\s*\};)"#,
    r#"=\s*function([\S\s]*?\}\s*return [\W\w$]+?\.call\([\w$]+?,\"\"\)\s*\};)"#,
];

pub static REGEX_SIGNATURE_TIMESTAMP: &Lazy<Regex> = regex!("signatureTimestamp[=:](\\d+)");

pub static REGEX_SIGNATURE_FUNCTION: Lazy<Regex> = Lazy::new(|| {
    Regex::new(concat!(
        r#"(?:"#,
            // Pattern 1
            r#"\b[a-zA-Z0-9$]+&&\([a-zA-Z0-9$]+=([a-zA-Z0-9$]{2,})\(decodeURIComponent\([a-zA-Z0-9$]+\)\)\)"#,
            r#"|"#,
            // Pattern 2
            r#"([a-zA-Z0-9$]+)\s*=\s*function\(\s*[a-zA-Z0-9$]+\s*\)\s*\{\s*[^}]+?\.split\(\s*""\s*\)[^}]+?\.join\(\s*""\s*\)"#,
            r#"|"#,
            // Pattern 3
            r#"(?:\b|[^a-zA-Z0-9$])([a-zA-Z0-9$]{2,})\s*=\s*function\(\s*a\s*\)\s*\{\s*a\s*=\s*a\.split\(\s*""\s*\)"#,
        r#")"#
    )).unwrap()
});
pub static REGEX_HELPER_OBJ_NAME: &Lazy<Regex> = regex!(r"([A-Za-z0-9_\$]{1,})=function\(");

pub static NSIG_FUNCTION_NAME: &str = "decrypt_nsig";
pub static SIG_FUNCTION_NAME: &str = "decrypt_sig";
