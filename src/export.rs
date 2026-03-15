//! Export pattern to MIDI or JSON file format.

use crate::patterns::Pattern;
use wasm_bindgen::prelude::*;
use web_sys::window;

/// Saved file format (JSON) for load/save
#[derive(serde::Serialize, serde::Deserialize)]
pub struct SessionFile {
    pub version: u32,
    pub code: String,
    pub bpm: u32,
}

/// Trigger browser download of a file
fn download_blob(data: &[u8], filename: &str, mime: &str) -> Result<(), JsValue> {
    let window = window().ok_or("no window")?;
    let document = window.document().ok_or("no document")?;
    let arr = js_sys::Uint8Array::from(data);
    let opts = web_sys::BlobPropertyBag::new();
    opts.set_type(mime);
    let blob = web_sys::Blob::new_with_u8_array_sequence_and_options(&arr.into(), &opts)?;
    let url = web_sys::Url::create_object_url_with_blob(&blob)?;
    let a = document.create_element("a")?;
    let a: web_sys::HtmlAnchorElement = a.dyn_into()?;
    a.set_href(&url);
    a.set_download(filename);
    let _ = a.set_attribute("style", "display:none");
    document.body().ok_or("no body")?.append_child(&a)?;
    a.click();
    document.body().unwrap().remove_child(&a)?;
    web_sys::Url::revoke_object_url(&url)?;
    Ok(())
}

/// Export pattern as JSON session file
pub fn export_json(code: &str, bpm: u32) -> Result<(), JsValue> {
    let session = SessionFile {
        version: 1,
        code: code.to_string(),
        bpm,
    };
    let json = serde_json::to_string_pretty(&session).unwrap();
    download_blob(json.as_bytes(), "songspark-session.json", "application/json")
}

/// Song file format (.jsong) for saving with metadata
#[derive(serde::Serialize)]
pub struct SongFileExport<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<&'a str>,
    pub code: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bpm: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub composer: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub link: Option<&'a str>,
}

/// Export as .jsong (song with metadata). Uses title for filename when present.
pub fn export_song(
    code: &str,
    bpm: u32,
    title: Option<&str>,
    composer: Option<&str>,
    description: Option<&str>,
    link: Option<&str>,
) -> Result<(), JsValue> {
    let song = SongFileExport {
        title,
        code,
        bpm: Some(bpm),
        composer,
        description,
        link,
    };
    let json = serde_json::to_string_pretty(&song).unwrap();
    let filename = title
        .filter(|s| !s.is_empty())
        .map(|t| format!("{}.jsong", t.replace(['/', '\\', ':', '*', '?', '"', '<', '>', '|'], "-").trim()))
        .unwrap_or_else(|| "song.jsong".to_string());
    download_blob(json.as_bytes(), &filename, "application/json")
}

/// Export pattern as MIDI file (simple format 0)
pub fn export_midi(pattern: &Pattern, bpm: u32) -> Result<(), JsValue> {
    let ppq = 480u16; // pulses per quarter note
    let mut data = Vec::new();

    // Header chunk: "MThd" + 6 bytes
    data.extend_from_slice(b"MThd");
    data.extend_from_slice(&6u32.to_be_bytes()[1..]); // length 6
    data.extend_from_slice(&0u16.to_be_bytes()); // format 0
    data.extend_from_slice(&1u16.to_be_bytes()); // 1 track
    data.extend_from_slice(&ppq.to_be_bytes());

    // Track chunk
    let mut track = Vec::new();
    let microsec_per_beat = 60_000_000 / bpm as u32;
    track.extend_from_slice(&0u8.to_be_bytes());
    track.extend_from_slice(&0xFFu8.to_be_bytes());
    track.extend_from_slice(&0x51u8.to_be_bytes());
    track.extend_from_slice(&3u8.to_be_bytes());
    track.push((microsec_per_beat >> 16) as u8);
    track.push((microsec_per_beat >> 8) as u8);
    track.push(microsec_per_beat as u8);

    // Note events: map samples to MIDI notes (C3 = 48, D3 = 50, etc.)
    let ticks_per_beat = ppq as u32;
    let ticks_per_step = ticks_per_beat / 4;
    let mut last_tick: u32 = 0;

    for ev in &pattern.events {
        let tick = (ev.time * ticks_per_step as f32) as u32;
        let delta = tick.saturating_sub(last_tick);
        write_var_len(&mut track, delta);

        let note = sample_to_midi(&ev.sample);
        track.push(0x90); // note on, ch 0
        track.push(note);
        track.push(80); // velocity

        last_tick = tick;
    }

    write_var_len(&mut track, ticks_per_beat);
    track.push(0xFF);
    track.push(0x2F);
    track.push(0); // end of track

    let track_len = track.len() as u32;
    data.extend_from_slice(b"MTrk");
    data.extend_from_slice(&track_len.to_be_bytes());
    data.extend_from_slice(&track);

    download_blob(&data, "songspark-export.mid", "audio/midi")
}

fn write_var_len(buf: &mut Vec<u8>, mut v: u32) {
    let mut tmp = [(v & 0x7F) as u8; 4];
    let mut n = 0;
    while v > 0 {
        tmp[n] = (v & 0x7F) as u8;
        v >>= 7;
        n += 1;
    }
    for i in (0..n).rev() {
        if i > 0 {
            buf.push(tmp[i] | 0x80);
        } else {
            buf.push(tmp[i]);
        }
    }
    if n == 0 {
        buf.push(0);
    }
}

fn sample_to_midi(sample: &str) -> u8 {
    let h = sample.bytes().fold(0u32, |a, b| a.wrapping_add(b as u32));
    (48 + (h % 24) as u8).clamp(36, 84)
}
