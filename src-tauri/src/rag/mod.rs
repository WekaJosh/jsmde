pub mod chunk;
pub mod embed;
pub mod index;
pub mod search;

pub const DEFAULT_MODEL: &str = "nomic-embed-text";
pub const EMBED_ACCOUNT: &str = "rag:enabled";

pub fn is_enabled() -> bool {
    crate::keychain::get_secret(EMBED_ACCOUNT)
        .ok()
        .flatten()
        .map(|v| v == "1")
        .unwrap_or(false)
}

pub fn set_enabled(value: bool) -> Result<(), String> {
    if value {
        crate::keychain::set_secret(EMBED_ACCOUNT, "1")
    } else {
        crate::keychain::delete_secret(EMBED_ACCOUNT)
    }
}
