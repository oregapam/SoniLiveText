use crate::errors::SonioxWindowsErrors;
use crate::types::settings::SettingsApp;
use crate::types::soniox::SonioxTranscriptionRequest;

pub trait SonioxMode {
    fn create_request<'a>(&self, settings: &'a SettingsApp) -> Result<SonioxTranscriptionRequest<'a>, SonioxWindowsErrors>;
}
