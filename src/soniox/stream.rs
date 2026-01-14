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
    'stream: loop {
        let url = URL.into_client_request()?;
        let (ws_stream, _) = connect_async(url).await?;
        let (mut write, mut read) = ws_stream.split();
        write
            .send(Message::Text(Utf8Bytes::try_from(bytes.clone())?))
            .await?;

        let tx_subs = tx_transcription.clone();
        let reader = async move {
            while let Some(msg) = read.next().await {
                if let Message::Text(txt) = msg? {
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

                    let response: SonioxTranscriptionResponse = serde_json::from_str(&txt)?;
                    let _ = tx_subs.send(response);
                }
            }
            <Result<(), SonioxWindowsErrors>>::Ok(())
        };

        tokio::spawn(async move {
            let _ = reader
                .await
                .inspect_err(|err| log::error!("error during read message: {}", err));
        });

        while let Some(message) = rx_audio.recv().await {
            match message {
                AudioMessage::Audio(buffer) => {
                    if buffer.is_empty() {
                        break;
                    }
                    let mut pcm16 = Vec::with_capacity(buffer.len() * 2);
                    for s in buffer {
                        let sample = (s.clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
                        pcm16.extend_from_slice(&sample.to_le_bytes());
                    }

                    let result = write.send(Message::Binary(Bytes::from(pcm16))).await;

                    if let Err(err) = result {
                        log::error!("error during sent binary -> {:?}", err);
                        continue 'stream;
                    }
                }
                AudioMessage::Stop => {
                    let _ = write.send(Message::Binary(Bytes::new())).await;
                    break 'stream;
                }
            }
        }

        let _ = write
            .send(Message::Binary(Bytes::new()))
            .await
            .inspect_err(|err| log::error!("error during write message: {}", err));
        break 'stream;
    }

    Ok(())
}

pub async fn start_soniox_stream(
    settings: &SettingsApp,
    tx_transcription: UnboundedSender<SonioxTranscriptionResponse>,
    rx_audio: UnboundedReceiver<AudioMessage>,
) -> Result<(), SonioxWindowsErrors> {
    // START OF REFACTOR: Select Mode
    let request = if settings.enable_translate() {
        let mode = TranslateMode;
        mode.create_request(settings)?
    } else {
        let mode = TranscribeMode;
        mode.create_request(settings)?
    };
    // END OF REFACTOR

    let bytes = serde_json::to_vec(&request)?;

    log::info!("Started Soniox stream!");
    log::info!("Starting to listen websocket stream Soniox...");
    listen_soniox_stream(bytes, tx_transcription, rx_audio, settings.enable_raw_logging()).await
}
