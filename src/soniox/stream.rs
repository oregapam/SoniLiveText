use crate::errors::SonioxWindowsErrors;
use crate::soniox::URL;
use crate::soniox::modes::SonioxMode;
use crate::soniox::transcribe_mode::TranscribeMode;
use crate::soniox::translate_mode::TranslateMode;
use crate::types::audio::AudioMessage;
use crate::types::settings::SettingsApp;
use crate::types::soniox::SonioxTranscriptionResponse;
use futures_util::{SinkExt, StreamExt};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio_tungstenite::connect_async;
use tungstenite::client::IntoClientRequest;
use tungstenite::{Bytes, Message, Utf8Bytes};
use std::fs::OpenOptions;
use std::io::Write;

async fn listen_soniox_stream(
    bytes: Vec<u8>,
    tx_transcription: UnboundedSender<SonioxTranscriptionResponse>,
    mut rx_audio: UnboundedReceiver<AudioMessage>,
    enable_raw_logging: bool,
) -> Result<(), SonioxWindowsErrors> {
    log::debug!("listen_soniox_stream: START");
    'stream: loop {
        log::debug!("listen_soniox_stream: Connecting to URL...");
        let url = URL.into_client_request()?;
        let (ws_stream, _) = match connect_async(url).await {
            Ok(v) => v,
            Err(e) => {
                log::error!("listen_soniox_stream: Connect FAILED: {:?}", e);
                return Err(SonioxWindowsErrors::Internal(e.to_string()));
            }
        };
        log::debug!("listen_soniox_stream: Connected!");
        
        let (mut write, mut read) = ws_stream.split();
        let json_str = String::from_utf8_lossy(&bytes);
        log::debug!("listen_soniox_stream: Sending JSON: {}", json_str);
        if let Err(e) = write.send(Message::Text(Utf8Bytes::try_from(bytes.clone())?)).await {
             log::error!("listen_soniox_stream: Failed to send initial JSON: {:?}", e);
             return Err(SonioxWindowsErrors::Internal(e.to_string()));
        }
        log::debug!("listen_soniox_stream: Initial JSON Sent.");

        let tx_subs = tx_transcription.clone();
        let reader = async move {
            log::debug!("listen_soniox_stream: Reader Task Started.");
            while let Some(msg) = read.next().await {
                match msg {
                     Ok(Message::Text(txt)) => {
                        log::debug!("Received Soniox Message: {}", txt);
                        // Log raw raw data to file
                        if enable_raw_logging {
                            if let Ok(mut file) = OpenOptions::new()
                                .create(true)
                                .append(true)
                                .open("raw_data.log") 
                            {
                                let _ = writeln!(file, "{}", txt);
                            }
                        }

                        if let Ok(response) = serde_json::from_str::<SonioxTranscriptionResponse>(&txt) {
                             let _ = tx_subs.send(response);
                        } else {
                             log::warn!("Failed to parse Soniox response: {}", txt);
                        }
                     },
                     Ok(Message::Close(c)) => {
                         log::debug!("listen_soniox_stream: Server sent CLOSE: {:?}", c);
                         break;
                     },
                     Err(e) => {
                         log::error!("listen_soniox_stream: Read Error: {:?}", e);
                         break;
                     }
                     _ => {} // Ignore Ping/Pong/Binary
                }
            }
            log::debug!("listen_soniox_stream: Reader Task FINISHED (Socket closed?).");
            <Result<(), SonioxWindowsErrors>>::Ok(())
        };

        tokio::spawn(async move {
            let _ = reader
                .await
                .inspect_err(|err| log::error!("error during read message: {}", err));
        });

        log::debug!("listen_soniox_stream: Starting Audio Loop...");
        while let Some(message) = rx_audio.recv().await {
            match message {
                AudioMessage::Audio(buffer) => {
                    if buffer.is_empty() {
                        log::warn!("listen_soniox_stream: Received EMPTY BUFFER. Breaking loop (Original Logic).");
                        break;
                    }
                    // Debug: Log every Nth packet to ensure flow? 
                    // No, too spammy.
                    
                    let mut pcm16 = Vec::with_capacity(buffer.len() * 2);
                    for s in buffer {
                        let sample = (s.clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
                        pcm16.extend_from_slice(&sample.to_le_bytes());
                    }

                    let result = write.send(Message::Binary(Bytes::from(pcm16))).await;
                    
                    // Very verbose, but necessary for now
                    // log::info!("listen_soniox_stream: Sent binary packet.");

                    if let Err(err) = result {
                        log::error!("listen_soniox_stream: error during sent binary -> {:?}. Reconnecting...", err);
                        continue 'stream;
                    }
                }
                AudioMessage::Stop => {
                    log::debug!("listen_soniox_stream: Received STOP message. Closing stream.");
                    let _ = write.send(Message::Binary(Bytes::new())).await;
                    break 'stream;
                }
            }
        }
        
        log::debug!("listen_soniox_stream: RX_AUDIO loop finished (Sender dropped or Break).");

        let _ = write
            .send(Message::Binary(Bytes::new()))
            .await
            .inspect_err(|err| log::error!("error during write message: {}", err));
        break 'stream;
    }

    log::debug!("listen_soniox_stream: RETURNING Ok. Stream Ended.");
    Ok(())
}

pub async fn start_soniox_stream(
    settings: &SettingsApp,
    tx_transcription: UnboundedSender<SonioxTranscriptionResponse>,
    rx_audio: UnboundedReceiver<AudioMessage>,
) -> Result<(), SonioxWindowsErrors> {
    // START OF REFACTOR: Select Mode
    
    // Determine Audio Format (The "Deep Research" Fix)
    // We lift this logic OUT of the mode and OUT of the request builder.
    // It is now strictly decided here before any request is formed.
    let (sample_rate, channels) = if settings.audio_input().trim() == "both" {
        log::debug!("start_soniox_stream: 'both' mode detected -> Forcing 16000Hz Mono");
        (16000, 1)
    } else {
         use wasapi::{DeviceEnumerator, Direction, initialize_mta};
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
        let sr = format.get_samplespersec();
        let ch = format.get_nchannels();
        log::info!("start_soniox_stream: Single device mode -> Detected {}Hz {}ch", sr, ch);
        (sr, ch)
    };
    
    let audio_format = (sample_rate, channels);

    let request = if settings.enable_translate() {
        let mode = TranslateMode;
        mode.create_request(settings, audio_format)?
    } else {
        let mode = TranscribeMode;
        mode.create_request(settings, audio_format)?
    };
    // END OF REFACTOR

    let bytes = serde_json::to_vec(&request)?;

    log::debug!("Started Soniox stream!");
    log::debug!("Starting to listen websocket stream Soniox...");
    listen_soniox_stream(bytes, tx_transcription, rx_audio, settings.enable_raw_logging()).await
}
