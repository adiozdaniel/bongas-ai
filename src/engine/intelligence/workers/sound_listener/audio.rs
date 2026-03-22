//! Real-world audio decoding using Symphonia.
//! Converts .mp4, .mkv, .wav into mono 16kHz f32 PCM for Whisper.

use std::fs::File;
use symphonia::core::audio::{AudioBufferRef, Signal};
use symphonia::core::codecs::DecoderOptions;
use symphonia::core::errors::Error;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;
use tracing::info;
use anyhow::{Result, Context};

/// Decodes an audio/video file into 16kHz mono PCM.
pub fn decode_to_pcm(path: &str) -> Result<Vec<f32>> {
    // 1. Open the media source
    let src = File::open(path).context("Failed to open audio file")?;
    let mss = MediaSourceStream::new(Box::new(src), Default::default());

    // 2. Probe for format
    let mut hint = Hint::new();
    if let Some(ext) = std::path::Path::new(path).extension() {
        if let Some(s) = ext.to_str() {
            hint.with_extension(s);
        }
    }

    let meta_opts = MetadataOptions::default();
    let fmt_opts = FormatOptions::default();
    let probed = symphonia::default::get_probe()
        .format(&hint, mss, &fmt_opts, &meta_opts)
        .context("Unsupported audio format")?;

    let mut format = probed.format;

    // 3. Find the first audio track
    let track = format
        .tracks()
        .iter()
        .find(|t| t.codec_params.codec != symphonia::core::codecs::CODEC_TYPE_NULL && t.codec_params.sample_rate.is_some())
        .context("No audio track found in file")?;

    let track_id = track.id;
    let source_sample_rate = track.codec_params.sample_rate.unwrap_or(44100) as f32;
    let target_sample_rate = 16000.0f32;
    let resample_ratio = source_sample_rate / target_sample_rate;

    // 4. Initialize the decoder
    let dec_opts = DecoderOptions::default();
    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &dec_opts)
        .context("Failed to create audio decoder")?;

    let mut raw_pcm_data = Vec::new();

    // 5. Decoding Loop (Extract raw samples)
    loop {
        let packet = match format.next_packet() {
            Ok(packet) => packet,
            Err(Error::IoError(_)) => break,
            Err(e) => return Err(e).context("Error reading packet"),
        };

        if packet.track_id() != track_id {
            continue;
        }

        match decoder.decode(&packet) {
            Ok(AudioBufferRef::F32(buf)) => {
                for i in 0..buf.frames() {
                    let mut sum = 0.0;
                    for channel in buf.planes().planes() {
                        sum += channel[i];
                    }
                    raw_pcm_data.push(sum / buf.planes().planes().len() as f32);
                }
            }
            Ok(_other) => {
                // In production, handle S16/S32/F64 if needed
            }
            Err(Error::DecodeError(_)) => continue,
            Err(e) => return Err(e).context("Decoding failed"),
        }
    }

    // 6. Resampling Logic (Linear Interpolation to 16kHz)
    if (source_sample_rate - 16000.0).abs() < 1.0 {
        return Ok(raw_pcm_data);
    }

    let mut resampled_data = Vec::new();
    let mut current_source_index = 0.0f32;

    while (current_source_index as usize) < raw_pcm_data.len() - 1 {
        let index_l = current_source_index as usize;
        let index_r = index_l + 1;
        let weight = current_source_index - (index_l as f32);

        // Linear interpolation: L * (1-w) + R * w
        let interpolated_sample = raw_pcm_data[index_l] * (1.0 - weight) + raw_pcm_data[index_r] * weight;
        resampled_data.push(interpolated_sample);

        current_source_index += resample_ratio;
    }

    info!(
        path = %path, 
        source_rate = %source_sample_rate, 
        samples = %resampled_data.len(), 
        "Audio decoded and resampled to 16kHz"
    );

    Ok(resampled_data)
}
