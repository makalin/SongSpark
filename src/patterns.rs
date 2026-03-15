use serde::{Serialize, Deserialize};

/// True if the token looks like a note name (e.g. A4, Bb3, C#5). Used to route to synth.
fn is_note_name(token: &str) -> bool {
    let b = token.as_bytes();
    if b.is_empty() {
        return false;
    }
    if !matches!(b[0], b'A'..=b'G') {
        return false;
    }
    let mut i = 1;
    if i < b.len() && (b[i] == b'#' || b[i] == b'b') {
        i += 1;
    }
    while i < b.len() && b[i].is_ascii_digit() {
        i += 1;
    }
    i == b.len()
}

/// One track/channel: its own step density (e.g. @4 = quarter notes, @16 = 16ths) and steps.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Channel {
    /// Steps per bar (4 = quarter notes, 8 = 8ths, 16 = 16ths). Each step can have multiple events (via |).
    pub division: u32,
    /// steps[step_index] = events at that step (sample/note names, is_note, span).
    pub steps: Vec<Vec<Event>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Pattern {
    /// Channels (tracks). Empty = legacy; one or more = parallel tracks with own density.
    #[serde(default)]
    pub channels: Vec<Channel>,
    /// Flattened events for playback (derived from channels). Same format as before for scheduling.
    pub events: Vec<Event>,
    pub gain: f32,
    pub pan: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Event {
    /// Sample name (e.g. "bd", "sd") or note name (e.g. "A4", "C#5") depending on is_note.
    pub sample: String,
    pub time: f32,
    /// If true, sample holds a note name (A4, Bb3) and is played by synth; else it's a sample name.
    #[serde(default)]
    pub is_note: bool,
    /// Byte range (start, end) in the source code for highlighting during playback.
    #[serde(skip_serializing, default)]
    pub span: Option<(usize, usize)>,
}

#[allow(dead_code)]
impl Pattern {
    pub fn new() -> Self {
        Self {
            channels: Vec::new(),
            events: Vec::new(),
            gain: 1.0,
            pan: 0.0,
        }
    }

    /// Duration of the pattern in 16th-note steps (pattern time 0.25 = one 16th). With channels we use 1 bar = 16.
    pub fn duration_16ths(&self) -> f32 {
        if self.events.is_empty() {
            return 1.0;
        }
        let max_t = self.events.iter().map(|e| e.time).fold(0.0f32, f32::max);
        // One bar = 4 beats = 16 16ths; ensure at least one bar when we have channels
        let from_events = (max_t + 0.25) / 0.25;
        if !self.channels.is_empty() {
            (from_events).max(16.0)
        } else {
            from_events
        }
    }

    /// Flatten channels to events (time in beats: step_index * 4/division). One bar = 4 beats.
    fn flatten_channels(channels: &[Channel]) -> Vec<Event> {
        let mut out = Vec::new();
        for ch in channels {
            for (step_index, step_events) in ch.steps.iter().enumerate() {
                let time_beats = step_index as f32 * (4.0 / ch.division as f32);
                for ev in step_events {
                    out.push(Event {
                        sample: ev.sample.clone(),
                        time: time_beats,
                        is_note: ev.is_note,
                        span: ev.span,
                    });
                }
            }
        }
        out.sort_by(|a, b| a.time.partial_cmp(&b.time).unwrap_or(std::cmp::Ordering::Equal));
        out
    }

    /// Parse one line (bytes[line_start..line_end]): optional @N then tokens. Returns (division, steps).
    fn parse_line(bytes: &[u8], line_start: usize, line_end: usize) -> Result<(u32, Vec<Vec<Event>>), String> {
        let mut division = 16u32;
        let mut i = line_start;

        while i < line_end && (bytes[i] == b' ' || bytes[i] == b'\t') {
            i += 1;
        }
        if i + 1 < line_end && bytes[i] == b'@' && bytes[i + 1].is_ascii_digit() {
            i += 1;
            let start = i;
            while i < line_end && bytes[i].is_ascii_digit() {
                i += 1;
            }
            division = std::str::from_utf8(&bytes[start..i]).unwrap_or("16").parse::<u32>().unwrap_or(16);
            division = division.clamp(1, 64);
        }

        let mut steps: Vec<Vec<Event>> = Vec::new();
        while i < line_end {
            while i < line_end && (bytes[i] == b' ' || bytes[i] == b'\t') {
                i += 1;
            }
            if i >= line_end {
                break;
            }
            if (bytes[i] == b'/' && i + 1 < line_end && bytes[i + 1] == b'/') || bytes[i] == b'#' {
                break;
            }
            let mut step_events = Vec::new();
            loop {
                while i < line_end && bytes[i] == b'|' {
                    i += 1;
                }
                while i < line_end && (bytes[i] == b' ' || bytes[i] == b'\t') {
                    i += 1;
                }
                if i >= line_end || bytes[i] == b'/' || bytes[i] == b'#' {
                    break;
                }
                let start = i;
                while i < line_end
                    && !matches!(bytes[i], b' ' | b'\t' | b'|' | b'/')
                    && bytes[i] != b'#'
                {
                    i += 1;
                }
                let end = i;
                if start >= end {
                    break;
                }
                let token = std::str::from_utf8(&bytes[start..end]).unwrap_or("");
                let span = (start, end);
                let is_note = is_note_name(token);
                if let Some((sample, repeat_str)) = token.split_once('*') {
                    let repeat = repeat_str.parse::<usize>().map_err(|_| "Invalid repeat count")?;
                    for _ in 0..repeat {
                        step_events.push(Event {
                            sample: sample.to_string(),
                            time: 0.0,
                            is_note,
                            span: Some(span),
                        });
                        steps.push(step_events.clone());
                        step_events.clear();
                    }
                    continue;
                }
                step_events.push(Event {
                    sample: token.to_string(),
                    time: 0.0,
                    is_note,
                    span: Some(span),
                });
                let mut j = i;
                while j < line_end && (bytes[j] == b' ' || bytes[j] == b'\t') {
                    j += 1;
                }
                if j >= line_end || bytes[j] != b'|' {
                    steps.push(step_events);
                    break;
                }
                i = j;
            }
        }
        Ok((division, steps))
    }

    /// Parse pattern code. Supports:
    /// - Multiple lines = channels (tracks). Start a line with @N for steps per bar (4=quarters, 16=16ths). Default 16.
    /// - Comments: `//` and `#` to end of line.
    /// - Tokens: sample names, note names (A4, C#5), `name*repeat`, `a|b|c` for same step.
    pub fn parse(input: &str) -> Result<Self, String> {
        let bytes = input.as_bytes();
        let has_newline = bytes.contains(&b'\n');

        if has_newline {
            let mut channels = Vec::new();
            let mut line_start = 0;
            while line_start < bytes.len() {
                while line_start < bytes.len() && bytes[line_start] == b'\n' {
                    line_start += 1;
                }
                if line_start >= bytes.len() {
                    break;
                }
                let line_end = bytes[line_start..]
                    .iter()
                    .position(|&b| b == b'\n')
                    .map(|p| line_start + p)
                    .unwrap_or(bytes.len());
                let (division, steps) = Self::parse_line(bytes, line_start, line_end)?;
                if !steps.is_empty() {
                    channels.push(Channel { division, steps });
                }
                line_start = line_end;
            }
            let events = Self::flatten_channels(&channels);
            return Ok(Pattern {
                channels,
                events,
                gain: 1.0,
                pan: 0.0,
            });
        }

        let mut pattern = Pattern::new();
        let mut time = 0.0;
        let mut i = 0;

        while i < bytes.len() {
            // Skip whitespace (including newlines)
            while i < bytes.len() && bytes[i].is_ascii_whitespace() {
                i += 1;
            }
            if i >= bytes.len() {
                break;
            }
            // Comment: // or # to end of line
            if (bytes[i] == b'/' && i + 1 < bytes.len() && bytes[i + 1] == b'/')
                || bytes[i] == b'#'
            {
                if bytes[i] == b'/' {
                    i += 2;
                } else {
                    i += 1;
                }
                while i < bytes.len() && bytes[i] != b'\n' {
                    i += 1;
                }
                continue;
            }
            // Collect one or more tokens separated by | (same time slot, parallel)
            let mut tokens_with_spans: Vec<(String, (usize, usize))> = Vec::new();
            loop {
                // Skip optional | between tokens
                while i < bytes.len() && bytes[i] == b'|' {
                    i += 1;
                }
                while i < bytes.len() && bytes[i].is_ascii_whitespace() && bytes[i] != b'\n' {
                    i += 1;
                }
                if i >= bytes.len() {
                    break;
                }
                if bytes[i] == b'/' || bytes[i] == b'#' {
                    break;
                }
                // Read one token
                let start = i;
                while i < bytes.len()
                    && !bytes[i].is_ascii_whitespace()
                    && bytes[i] != b'/'
                    && bytes[i] != b'#'
                    && bytes[i] != b'|'
                {
                    i += 1;
                }
                let end = i;
                if start == end {
                    break;
                }
                let token = std::str::from_utf8(&bytes[start..end]).unwrap_or("");
                tokens_with_spans.push((token.to_string(), (start, end)));
                // If next non-space is not |, we're done with this slot
                let mut j = i;
                while j < bytes.len() && bytes[j].is_ascii_whitespace() && bytes[j] != b'\n' {
                    j += 1;
                }
                if j >= bytes.len() || bytes[j] != b'|' {
                    break;
                }
            }

            let slot_time = time;
            for (token, span) in &tokens_with_spans {
                let is_note = is_note_name(token);
                if let Some((sample, repeat_str)) = token.split_once('*') {
                    let repeat = repeat_str
                        .parse::<usize>()
                        .map_err(|_| "Invalid repeat count")?;
                    for k in 0..repeat {
                        pattern.events.push(Event {
                            sample: sample.to_string(),
                            time: slot_time + (k as f32) * 0.25,
                            is_note,
                            span: Some(*span),
                        });
                    }
                } else {
                    pattern.events.push(Event {
                        sample: token.clone(),
                        time: slot_time,
                        is_note,
                        span: Some(*span),
                    });
                }
            }
            let max_repeat = tokens_with_spans.iter().map(|(t, _)| {
                if let Some((_, r)) = t.split_once('*') {
                    r.parse::<usize>().unwrap_or(1)
                } else {
                    1
                }
            }).max().unwrap_or(1);
            time = slot_time + 0.25 * (max_repeat as f32);
        }

        Ok(pattern)
    }

    pub fn gain(mut self, gain: f32) -> Self {
        self.gain = gain;
        self
    }

    pub fn pan(mut self, pan: f32) -> Self {
        self.pan = pan;
        self
    }
} 