use crate::errors::SonioxWindowsErrors;

use crate::types::settings::SettingsApp;
use crate::types::soniox::{SonioxTranscriptionRequest, SonioxTranslationObject};
use wasapi::{DeviceEnumerator, Direction, initialize_mta};

pub(crate) fn create_request(
    settings: &'_ SettingsApp,
    audio_format: (u32, u16), // (sample_rate, channels)
) -> Result<SonioxTranscriptionRequest<'_>, SonioxWindowsErrors> {
    let (sample_rate, channels) = audio_format;
    
    // Log final used format for verification
    log::info!("create_request: Using Explicit Format -> Sample Rate: {}, Channels: {}", sample_rate, channels);
    let mut request = SonioxTranscriptionRequest {
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
    if settings.enable_translate() {
        request.translation = Some(SonioxTranslationObject {
            r#type: "one_way",
            target_language: Some(settings.target_language()),
            ..Default::default()
        });
    }

    Ok(request)
}
