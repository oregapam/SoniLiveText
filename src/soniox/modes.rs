use crate::errors::SonioxWindowsErrors;
use crate::types::settings::SettingsApp;
use crate::types::soniox::SonioxTranscriptionRequest;
use crate::soniox::state::TranscriptionState;
use crate::types::soniox::SonioxTranscriptionResponse;

pub trait SonioxMode {
    fn create_request<'a>(&self, settings: &'a SettingsApp) -> Result<SonioxTranscriptionRequest<'a>, SonioxWindowsErrors>;
    fn handle_incoming(&self, state: &mut TranscriptionState, response: SonioxTranscriptionResponse);
    fn process_event(&self, state: &mut TranscriptionState, response: SonioxTranscriptionResponse);
}
