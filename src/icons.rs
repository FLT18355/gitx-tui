pub const GITHUB: &str = "\u{f09b}";
pub const REPO: &str = "\u{f07b}";
pub const STAR: &str = "\u{f005}";
pub const SEARCH: &str = "\u{f002}";
pub const SETTINGS: &str = "\u{f013}";
pub const USER: &str = "\u{f007}";
pub const LOCK: &str = "\u{f023}";
pub const GLOBE: &str = "\u{f0ac}";
pub const FORK: &str = "\u{f126}";
pub const ISSUE: &str = "\u{f188}";
pub const CLOCK: &str = "\u{f017}";
pub const TAG: &str = "\u{f02b}";
pub const DOWNLOAD: &str = "\u{f019}";
pub const COPY: &str = "\u{f0c5}";
pub const EXTERNAL: &str = "\u{f08e}";
pub const REFRESH: &str = "\u{f021}";
pub const TRASH: &str = "\u{f1f8}";
pub const SAVE: &str = "\u{f0c7}";
pub const NETWORK: &str = "\u{f1eb}";
pub const DATABASE: &str = "\u{f1c0}";
pub const CODE: &str = "\u{f121}";
pub const PALETTE: &str = "\u{f1fc}";
pub const BACK: &str = "\u{f060}";
pub const CHECK: &str = "\u{f00c}";
pub const WARNING: &str = "\u{f071}";
pub const INFO: &str = "\u{f05a}";
pub const ARCHIVE: &str = "\u{f1c6}";
pub const TERMINAL: &str = "\u{f120}";
pub const LICENSE: &str = "\u{f24e}";
pub const BRANCH: &str = "\u{f126}";
pub const EYE: &str = "\u{f06e}";

pub fn label(icon: &str, text: &str) -> String {
    format!("{icon} {text}")
}
