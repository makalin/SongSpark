use wasm_bindgen::prelude::*;
use web_sys::{AudioContext, AudioBuffer, AudioBufferSourceNode, GainNode, StereoPannerNode, OscillatorNode, OscillatorType};
use std::collections::HashMap;

/// Convert note name (e.g. "A4", "C#5", "Bb3") to frequency in Hz. A4 = 440. Returns None if not a valid note.
pub fn note_name_to_freq(name: &str) -> Option<f64> {
    let b = name.as_bytes();
    if b.is_empty() || !matches!(b[0], b'A'..=b'G') {
        return None;
    }
    let semitone = match b[0] {
        b'C' => 0,
        b'D' => 2,
        b'E' => 4,
        b'F' => 5,
        b'G' => 7,
        b'A' => 9,
        b'B' => 11,
        _ => return None,
    };
    let mut i = 1;
    let mut semitone = semitone as i32;
    if i < b.len() && (b[i] == b'#' || b[i] == b'b') {
        if b[i] == b'#' {
            semitone += 1;
        } else {
            semitone -= 1;
        }
        i += 1;
    }
    let mut octave = 4i32;
    if i < b.len() && b[i].is_ascii_digit() {
        let mut o = 0i32;
        while i < b.len() && b[i].is_ascii_digit() {
            o = o * 10 + (b[i] - b'0') as i32;
            i += 1;
        }
        octave = o.clamp(0, 8);
    }
    if i != b.len() {
        return None;
    }
    let midi = octave * 12 + semitone;
    let midi = midi.clamp(0, 127);
    // A4 = 440 Hz, MIDI 69
    Some(440.0 * 2f64.powf((midi - 69) as f64 / 12.0))
}

#[derive(Clone)]
pub struct AudioEngine {
    pub(crate) context: AudioContext,
    /// pack_name -> (sample_name -> buffer)
    samples: HashMap<String, HashMap<String, AudioBuffer>>,
    active_pack: String,
    /// Master gain node (all playback goes through this).
    master_gain: GainNode,
}

impl Default for AudioEngine {
    fn default() -> Self {
        Self::new().expect("AudioContext::new failed")
    }
}

impl PartialEq for AudioEngine {
    fn eq(&self, _other: &Self) -> bool {
        true
    }
}

impl AudioEngine {
    pub fn new() -> Result<Self, JsValue> {
        let context = AudioContext::new()?;
        let master_gain = GainNode::new(&context)?;
        master_gain.gain().set_value(1.0);
        master_gain.connect_with_audio_node(&context.destination())?;
        Ok(Self {
            context,
            samples: HashMap::new(),
            active_pack: String::new(),
            master_gain,
        })
    }

    pub fn load_sample(&mut self, pack: &str, name: &str, buffer: AudioBuffer) {
        self.samples
            .entry(pack.to_string())
            .or_default()
            .insert(name.to_string(), buffer);
        if self.active_pack.is_empty() {
            self.active_pack = pack.to_string();
        }
    }

    pub fn set_active_pack(&mut self, pack: &str) {
        self.active_pack = pack.to_string();
    }

    /// Remove a pack and its samples (e.g. before reloading the same pack).
    pub fn clear_pack(&mut self, pack: &str) {
        self.samples.remove(pack);
        if self.active_pack == pack {
            self.active_pack = self.samples.keys().next().cloned().unwrap_or_default();
        }
    }

    #[allow(dead_code)]
    pub fn active_pack(&self) -> &str {
        &self.active_pack
    }

    #[allow(dead_code)]
    pub fn pack_names(&self) -> Vec<String> {
        self.samples.keys().cloned().collect()
    }

    /// Resume the audio context (required after user gesture in browsers; safe to call when already running).
    pub fn resume(&self) {
        let _ = self.context.resume();
    }

    /// Current time in seconds (for scheduling).
    pub fn current_time(&self) -> f64 {
        self.context.current_time()
    }

    /// Play a sample at a given time (seconds on the audio timeline).
    pub fn play_sample_at(&self, name: &str, gain: f32, pan: f32, when: f64) -> Result<(), JsValue> {
        let buffer = self
            .samples
            .get(&self.active_pack)
            .and_then(|m| m.get(name));
        if let Some(buffer) = buffer {
            let source = AudioBufferSourceNode::new(&self.context)?;
            source.set_buffer(Some(buffer));

            let gain_node = GainNode::new(&self.context)?;
            gain_node.gain().set_value(gain);

            let panner = StereoPannerNode::new(&self.context)?;
            panner.pan().set_value(pan);

            source.connect_with_audio_node(&gain_node)?;
            gain_node.connect_with_audio_node(&panner)?;
            panner.connect_with_audio_node(&self.master_gain)?;

            source.start_with_when(when)?;
        }
        Ok(())
    }

    pub fn play_sample(&self, name: &str, gain: f32, pan: f32) -> Result<(), JsValue> {
        self.play_sample_at(name, gain, pan, self.context.current_time())
    }

    /// Play a synth note (sine) at a given time. Note name e.g. "A4", "C#5", "Bb3". Duration in seconds.
    pub fn play_synth_note_at(&self, note_name: &str, gain: f32, pan: f32, when: f64, duration_sec: f64) -> Result<(), JsValue> {
        let Some(freq) = note_name_to_freq(note_name) else {
            return Ok(());
        };
        let osc = OscillatorNode::new(&self.context)?;
        osc.set_type(OscillatorType::Sine);
        osc.frequency().set_value(freq as f32);

        let gain_node = GainNode::new(&self.context)?;
        gain_node.gain().set_value(gain);

        let panner = StereoPannerNode::new(&self.context)?;
        panner.pan().set_value(pan);

        osc.connect_with_audio_node(&gain_node)?;
        gain_node.connect_with_audio_node(&panner)?;
        panner.connect_with_audio_node(&self.master_gain)?;

        osc.start_with_when(when)?;
        osc.stop_with_when(when + duration_sec)?;
        Ok(())
    }

    /// Set master output volume (0.0 .. 2.0, 1.0 = unity).
    pub fn set_master_volume(&self, value: f32) {
        self.master_gain.gain().set_value(value.clamp(0.0, 2.0));
    }
}
