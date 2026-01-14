pub(crate) mod state;
// pub(crate) mod request; // Deprecated/Internal now, but kept if needed by other legacy. 
// Actually I'll keep it for now but maybe I don't need to export it if stream uses modes.
// pub(crate) mod request; 
pub(crate) mod stream;
pub(crate) mod modes;
pub(crate) mod transcribe_mode;
pub(crate) mod translate_mode;
pub mod validation;

pub const URL: &str = "wss://stt-rt.soniox.com/transcribe-websocket";
