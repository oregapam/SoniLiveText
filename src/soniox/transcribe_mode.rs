use crate::errors::SonioxWindowsErrors;
use crate::types::settings::SettingsApp;
use crate::types::soniox::SonioxTranscriptionRequest;
use crate::soniox::modes::SonioxMode;
use wasapi::{DeviceEnumerator, Direction, initialize_mta};
// use crate::soniox::request::get_audio_config; // Removed: Logic duplicated locally. 
// Actually, let's keep it simple first and duplicate if needed or extract a helper.
// The original create_request had device initialization. It's better to extract that helper.

pub struct TranscribeMode;

impl SonioxMode for TranscribeMode {
    fn create_request<'a>(&self, settings: &'a SettingsApp) -> Result<SonioxTranscriptionRequest<'a>, SonioxWindowsErrors> {
        // Reuse logic from request.rs but specific for transcription
        // Note: For now, I will assume we can refactor request.rs to export a helper for common audio setup
        // OR I will duplicate the common setup to ensure clean decoupling as requested.
        // Duplication is safer for "splitting" to avoid shared dependencies we might want to diverge later.
        
        let _ = initialize_mta().ok(); // Ignoring result as in original
        let enumerator = DeviceEnumerator::new()?;
        
        let direction = if settings.audio_input() == "microphone" {
            Direction::Capture
        } else {
            Direction::Render
        };
        
        let device = enumerator.get_default_device(&direction)?;
        let audio_client = device.get_iaudioclient()?;
        let format = audio_client.get_mixformat()?;
        let sample_rate = format.get_samplespersec();
        let channels = format.get_nchannels();
        
        let request = SonioxTranscriptionRequest {
            api_key: settings.api_key(),
            model: settings.model(),
            audio_format: "pcm_s16le",
            sample_rate: Some(sample_rate),
            num_channels: Some(channels as u32),
            context: Some(settings.context()),
            language_hints: settings.language_hints(),
            enable_speaker_diarization: Some(settings.enable_speakers()),
            enable_non_final_tokens: Some(true),
            ..Default::default()
        };

        // No translation object for TranscribeMode
        Ok(request)
    }
}
