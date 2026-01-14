use crate::errors::SonioxWindowsErrors;
use crate::types::settings::SettingsApp;
use crate::types::soniox::{SonioxTranscriptionRequest, SonioxTranslationObject};
use crate::soniox::modes::SonioxMode;
use wasapi::{DeviceEnumerator, Direction, initialize_mta};

pub struct TranslateMode;

impl SonioxMode for TranslateMode {
    fn create_request<'a>(&self, settings: &'a SettingsApp) -> Result<SonioxTranscriptionRequest<'a>, SonioxWindowsErrors> {
        let _ = initialize_mta().ok();
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
        
        let translation_obj = SonioxTranslationObject {
            r#type: "one_way",
            target_language: Some(settings.target_language()),
            ..Default::default()
        };

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
            translation: Some(translation_obj),
            ..Default::default()
        };

        Ok(request)
    }
}
