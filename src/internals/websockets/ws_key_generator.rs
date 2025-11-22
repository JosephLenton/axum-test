use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use uuid::Uuid;

/// Generates a random key for use, that is base 64 encoded for use over HTTP.
pub fn generate_ws_key() -> String {
    STANDARD.encode(Uuid::new_v4().as_bytes())
}
