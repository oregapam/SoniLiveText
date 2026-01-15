use crate::errors::SonioxWindowsErrors;
use crate::types::audio::AudioMessage;
use bytemuck::cast_slice;
use std::thread::{self, sleep};
use std::time::Duration;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use wasapi::{DeviceEnumerator, Direction, StreamMode, initialize_mta};
use std::sync::mpsc::{channel, TryRecvError};

#[derive(Debug)]
enum StartCaptureType {
    Microphone,
    Loopback,
}

pub fn start_capture_audio(
    tx_audio: UnboundedSender<AudioMessage>,
    rx_stop: UnboundedReceiver<bool>,
    input_mode: &str,
) -> Result<(), SonioxWindowsErrors> {
    if input_mode == "both" {
        start_dual_capture(tx_audio, rx_stop)
    } else {
        start_single_capture(tx_audio, rx_stop, input_mode)
    }
}

fn start_single_capture(
    tx_audio: UnboundedSender<AudioMessage>,
    mut rx_stop: UnboundedReceiver<bool>,
    input_mode: &str,
) -> Result<(), SonioxWindowsErrors> {
    initialize_mta()
        .ok()
        .map_err(|_| SonioxWindowsErrors::Internal("".to_string()))?;
    let enumerator = DeviceEnumerator::new()?;
    
    let direction = if input_mode == "microphone" {
        Direction::Capture
    } else {
        Direction::Render
    };
    
    let device = enumerator.get_default_device(&direction)?;
    let mut audio_client = device.get_iaudioclient()?;
    let format = audio_client.get_mixformat()?;
    let bytes_per_frame = format.get_blockalign() as usize;

    let mode = StreamMode::PollingShared {
        autoconvert: false,
        buffer_duration_hns: 1_000_000,
    };
    audio_client.initialize_client(&format, &Direction::Capture, &mode)?;

    let capture = audio_client.get_audiocaptureclient()?;
    audio_client.start_stream()?;

    log::info!("Started single audio stream: {}", input_mode);
    loop {
        if let Ok(true) = rx_stop.try_recv() {
            log::info!("Audio thread terminated!");
            break;
        }

        let frames = match capture.get_next_packet_size()? {
            Some(f) if f > 0 => f,
            _ => {
                sleep(Duration::from_millis(50));
                continue;
            }
        };

        let mut buffer = vec![0u8; frames as usize * bytes_per_frame];
        let _ = capture.read_from_device(&mut buffer)?;

        let final_buffer: Vec<f32> = if !buffer.len().is_multiple_of(4) {
            log::warn!("Buffer size not multiple of 4: {}", buffer.len());
            Vec::new()
        } else {
            cast_slice::<u8, f32>(&buffer).to_vec()
        };
        let result = tx_audio.send(AudioMessage::Audio(final_buffer));

        if let Err(err) = result {
            log::info!("Audio thread terminated, error: {:?}", err);
            break;
        }
    }

    audio_client.stop_stream()?;
    let _ = tx_audio.send(AudioMessage::Stop);
    Ok(())
}

fn start_dual_capture(
    tx_audio: UnboundedSender<AudioMessage>,
    mut rx_stop: UnboundedReceiver<bool>,
) -> Result<(), SonioxWindowsErrors> {
    initialize_mta()
        .ok()
        .map_err(|_| SonioxWindowsErrors::Internal("".to_string()))?;

    log::info!("Initializing Dual Capture Mode...");

    let (tx_mic_internal, rx_mic_internal) = channel::<Vec<f32>>();
    let (tx_sys_internal, rx_sys_internal) = channel::<Vec<f32>>();

    // --- 1. Start Mic Thread ---
    thread::spawn(move || {
        log::info!("Starting Mic Thread...");
        if let Err(e) = run_capture_loop(StartCaptureType::Microphone, tx_mic_internal) {
            log::error!("Mic capture thread FAILED: {:?}", e);
        } else {
            log::info!("Mic capture thread finished normally");
        }
    });

    // --- 2. Start System Thread ---
    thread::spawn(move || {
        log::info!("Starting System Thread...");
        if let Err(e) = run_capture_loop(StartCaptureType::Loopback, tx_sys_internal) {
            log::error!("System capture thread FAILED: {:?}", e);
        } else {
             log::info!("System capture thread finished normally");
        }
    });

    log::info!("Mixer Loop Starting...");

    // Initialize WAV writer
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: 16000,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut wav_writer = match hound::WavWriter::create("debug_audio.wav", spec) {
        Ok(w) => Some(w),
        Err(e) => {
            log::error!("Failed to create debug_audio.wav: {}", e);
            None
        }
    };

    // --- 3. Mixer Loop ---
    let mut sys_buffer: Vec<f32> = Vec::new();
    const MAX_SYS_BUFFER_SIZE: usize = 48000 * 2; 

    loop {
        if let Ok(true) = rx_stop.try_recv() {
            log::info!("Dual audio mixer terminated via signal!");
            break;
        }

        // Wait for Mic (Master Clock)
        let mic_chunk = match rx_mic_internal.recv() {
            Ok(chunk) => chunk,
            Err(_) => {
                log::error!("CRITICAL: Mic channel closed unexpectedly (thread died?). Mixer stopping.");
                break;
            }
        };

        // Drain all available system audio
        loop {
            match rx_sys_internal.try_recv() {
                Ok(mut chunk) => sys_buffer.append(&mut chunk),
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    log::warn!("System audio channel disconnected - continuing with Mic Only");
                    break; 
                }
            }
        }
        
        if sys_buffer.len() > MAX_SYS_BUFFER_SIZE {
             let excess = sys_buffer.len() - MAX_SYS_BUFFER_SIZE;
             sys_buffer.drain(0..excess);
        }



        // Mix
        let mut mixed_chunk: Vec<f32> = Vec::with_capacity(mic_chunk.len());
        let frames_to_mix = mic_chunk.len();
        
        let sys_part: Vec<f32> = if sys_buffer.len() >= frames_to_mix {
            sys_buffer.drain(0..frames_to_mix).collect()
        } else {
             // Silence padding
             let mut part = sys_buffer.drain(..).collect::<Vec<f32>>();
             part.resize(frames_to_mix, 0.0);
             part
        };

        let mut max_amp = 0.0f32;
        for i in 0..frames_to_mix {
            let mic_sample = mic_chunk[i];
            let sys_sample = sys_part[i];
            
            // Sum and clamp
            let sum = mic_sample + sys_sample;
            // Hard clamp
            let clamped = if sum > 1.0 { 1.0 } else if sum < -1.0 { -1.0 } else { sum };
            mixed_chunk.push(clamped);
            if clamped.abs() > max_amp { max_amp = clamped.abs(); }
        }

        // Reduced log frequency: log only if amp > 0.01 (silence is usually near 0)
        if max_amp > 0.001 {
             log::info!("Mixer chunk positive. Max Amp: {}", max_amp);
        } else {
             // log::debug!("Mixer chunk silence.");
        }

        // Write to WAV for debugging
        if let Some(writer) = &mut wav_writer {
            for &sample in &mixed_chunk {
                // Convert f32 (-1.0 to 1.0) to i16 for WAV
                let amplitude = (sample * i16::MAX as f32).clamp(i16::MIN as f32, i16::MAX as f32) as i16;
                if let Err(e) = writer.write_sample(amplitude) {
                    log::error!("Failed to write sample to WAV: {}", e);
                }
            }
        }

        if mixed_chunk.is_empty() {
             log::warn!("Mixer produced empty chunk? Ignoring to prevent stream closure.");
             continue;
        }

        let result = tx_audio.send(AudioMessage::Audio(mixed_chunk));
        if let Err(err) = result {
             log::info!("Mixer thread send failed: {:?}", err);
             break;
        }
    }
    
    log::info!("Mixer Loop Exiting. Sending Stop.");
    let _ = tx_audio.send(AudioMessage::Stop);
    Ok(())
}

fn run_capture_loop(
    capture_type: StartCaptureType,
    tx: std::sync::mpsc::Sender<Vec<f32>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let _ = initialize_mta().ok(); 
    
    let enumerator = DeviceEnumerator::new()?;
    
    // Change: Use Role::Console (Default) for both to match single-mode behavior
    // Loopback is Render/Console. Mic is Capture/Console.
    let (direction, role) = match capture_type {
        StartCaptureType::Microphone => (Direction::Capture, wasapi::Role::Console),
        StartCaptureType::Loopback => (Direction::Render, wasapi::Role::Console), 
    };

    log::info!("[{:?}] Getting default device for Role::{:?}", capture_type, role);
    let device = enumerator.get_default_device_for_role(&direction, &role)?;
    let name = device.get_friendlyname()?;
    log::info!("[{:?}] Using device: {}", capture_type, name);

    let mut audio_client = device.get_iaudioclient()?;
    
    // Request specific format: 16k, 1 channel, f32
    // We rely on autoconvert: true
    let wave_format = wasapi::WaveFormat::new(
        32, 
        32, 
        &wasapi::SampleType::Float,
        16000, 
        1, 
        None 
    );
    
    log::info!("[{:?}] Initializing client with autoconvert=true, 16kHz Mono", capture_type);

    let mode = StreamMode::PollingShared {
        autoconvert: true,
        buffer_duration_hns: 1_000_000, 
    };

    audio_client.initialize_client(&wave_format, &Direction::Capture, &mode)?;
    let capture = audio_client.get_audiocaptureclient()?;
    audio_client.start_stream()?;
    log::info!("[{:?}] Stream started successfully!", capture_type);
    
    let bytes_per_frame = 4; // f32

    let mut first_packet = true;

    loop {
         let packet_size = match capture.get_next_packet_size() {
             Ok(Some(s)) => s,
             Ok(None) => {
                 sleep(Duration::from_millis(5));
                 continue;
             },
             Err(e) => {
                 log::error!("[{:?}] Capture error: {:?}", capture_type, e);
                 break;
             }
         };
         
         if packet_size == 0 {
             sleep(Duration::from_millis(5));
             continue;
         }
         
         if first_packet {
             log::info!("[{:?}] First packet received! Size: {}", capture_type, packet_size);
             first_packet = false;
         }

         let mut buffer = vec![0u8; packet_size as usize * bytes_per_frame];
         match capture.read_from_device(&mut buffer) {
             Ok(_) => {
                 if buffer.len() % 4 == 0 {
                      let float_data: Vec<f32> = cast_slice::<u8, f32>(&buffer).to_vec();
                      if tx.send(float_data).is_err() {
                          log::warn!("[{:?}] Receiver closed, stopping thread.", capture_type);
                          break; 
                      }
                 }
             },
             Err(e) => {
                 log::warn!("[{:?}] Read error: {:?}", capture_type, e);
                 break;
             }
         }
    }
    
    audio_client.stop_stream().ok();
    Ok(())
}
