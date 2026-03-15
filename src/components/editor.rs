use yew::prelude::*;
use wasm_bindgen::JsCast;
use gloo_net::http::Request;
use crate::patterns::Pattern;
use crate::presets::Preset;
use crate::export;
use serde::Deserialize;

#[derive(Properties, PartialEq)]
pub struct EditorProps {
    pub code: String,
    pub on_code_change: Callback<String>,
    pub on_pattern_change: Callback<Pattern>,
    pub pattern: Pattern,
    pub bpm: u32,
    #[prop_or_default]
    pub on_bpm_change: Option<Callback<u32>>,
    pub presets: Vec<Preset>,
    /// Indices into pattern.events that are currently playing (for highlight).
    #[prop_or_default]
    pub current_playing_indices: Vec<usize>,
}

/// Editable song metadata (loaded from .jsong or filled by user). Saveable as .jsong.
#[derive(Clone, Default, Deserialize, serde::Serialize)]
pub struct SongInfo {
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub bpm: Option<u32>,
    #[serde(default)]
    pub composer: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub link: Option<String>,
}

#[derive(Clone, Deserialize)]
struct ExampleIndexItem {
    id: String,
    title: String,
    file: String,
}

/// Song example format (.jsong): code + metadata. Same shape for file load and examples fetch.
#[derive(Deserialize)]
struct SongExampleFile {
    #[serde(default)]
    title: Option<String>,
    code: String,
    #[serde(default)]
    bpm: Option<u32>,
    #[serde(default)]
    composer: Option<String>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    link: Option<String>,
}

impl From<SongExampleFile> for SongInfo {
    fn from(f: SongExampleFile) -> Self {
        SongInfo {
            title: f.title,
            bpm: f.bpm,
            composer: f.composer,
            description: f.description,
            link: f.link,
        }
    }
}

/// Store raw input for display; use trim only when deciding None vs Some (empty field).
fn opt_value(s: String) -> Option<String> {
    if s.trim().is_empty() { None } else { Some(s) }
}

fn escape_html(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            _ => out.push(c),
        }
    }
    out
}

/// Ranges where code is comment (from // or # to end of line). Used for grey styling.
fn get_comment_ranges(code: &str) -> Vec<(usize, usize)> {
    let bytes = code.as_bytes();
    let mut ranges = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if (bytes[i] == b'/' && i + 1 < bytes.len() && bytes[i + 1] == b'/')
            || bytes[i] == b'#'
        {
            let start = i;
            if bytes[i] == b'/' {
                i += 2;
            } else {
                i += 1;
            }
            while i < bytes.len() && bytes[i] != b'\n' {
                i += 1;
            }
            ranges.push((start, i));
        } else {
            i += 1;
        }
    }
    ranges
}

/// Build (start, end, highlight) segments for code from merged highlight ranges.
fn build_highlight_segments(code: &str, mut ranges: Vec<(usize, usize)>) -> Vec<(usize, usize, bool)> {
    ranges.sort_by_key(|r| r.0);
    let mut merged = Vec::new();
    for (a, b) in ranges {
        if let Some((_, ref mut end)) = merged.last_mut() {
            if a <= *end {
                *end = (*end).max(b);
            } else {
                merged.push((a, b));
            }
        } else {
            merged.push((a, b));
        }
    }
    let mut segs = Vec::new();
    let mut pos = 0;
    for (a, b) in merged {
        if a > pos {
            segs.push((pos, a, false));
        }
        segs.push((a, b, true));
        pos = b;
    }
    if pos < code.len() {
        segs.push((pos, code.len(), false));
    }
    segs
}

/// Segment with (start, end, is_comment, is_playing). Covers whole code.
fn build_code_layer_segments(
    code: &str,
    comment_ranges: &[(usize, usize)],
    play_ranges: &[(usize, usize)],
) -> Vec<(usize, usize, bool, bool)> {
    let mut bounds = vec![0, code.len()];
    for &(a, b) in comment_ranges.iter().chain(play_ranges) {
        if a < code.len() {
            bounds.push(a);
        }
        if b <= code.len() && b > 0 {
            bounds.push(b);
        }
    }
    bounds.sort_unstable();
    bounds.dedup();
    let mut segs = Vec::new();
    for w in bounds.windows(2) {
        let (a, b) = (w[0], w[1]);
        if a >= b {
            continue;
        }
        let is_comment = comment_ranges.iter().any(|&(c, d)| c < b && d > a);
        let is_playing = play_ranges.iter().any(|&(c, d)| c < b && d > a);
        segs.push((a, b, is_comment, is_playing));
    }
    segs
}

#[function_component(Editor)]
pub fn editor(props: &EditorProps) -> Html {
    let file_input_ref = use_node_ref();
    let examples = use_state(|| Vec::<ExampleIndexItem>::new());
    // Editable song details (loaded from file/example or user-edited). Shown in collapsible section.
    let song_details = use_state(|| SongInfo::default());

    use_effect_with_deps(
        {
            let examples = examples.clone();
            move |_| {
                let examples = examples.clone();
                wasm_bindgen_futures::spawn_local(async move {
                    // Resolve relative to document so it works when app is in a subpath (e.g. /SongSpark/dist/)
                    let url = web_sys::window()
                        .and_then(|w| w.location().href().ok())
                        .and_then(|href| {
                            href.rfind('/').map(|i| format!("{}examples/index.json", &href[..=i]))
                        })
                        .unwrap_or_else(|| "examples/index.json".into());
                    if let Ok(resp) = Request::get(&url).send().await {
                        if resp.ok() {
                            if let Ok(list) = resp.json::<Vec<ExampleIndexItem>>().await {
                                examples.set(list);
                            }
                        }
                    }
                });
                || ()
            }
        },
        (),
    );

    let on_example_change = {
        let on_code_change = props.on_code_change.clone();
        let on_pattern_change = props.on_pattern_change.clone();
        let on_bpm_change = props.on_bpm_change.clone();
        let song_details = song_details.clone();
        let examples = examples.clone();
        Callback::from(move |e: Event| {
            let select: web_sys::HtmlSelectElement = e.target_unchecked_into();
            let val = select.value();
            if val.is_empty() {
                return;
            }
            let examples = (*examples).clone();
            let file = match examples.iter().find(|x| x.id == val) {
                Some(x) => x.file.clone(),
                None => return,
            };
            let on_code_change = on_code_change.clone();
            let on_pattern_change = on_pattern_change.clone();
            let on_bpm_change = on_bpm_change.clone();
            let song_details = song_details.clone();
            wasm_bindgen_futures::spawn_local(async move {
                let base = web_sys::window()
                    .and_then(|w| w.location().href().ok())
                    .and_then(|href| href.rfind('/').map(|i| href[..=i].to_string()))
                    .unwrap_or_else(|| "".into());
                let song_url = format!("{}examples/{}", base, file);
                if let Ok(resp) = Request::get(&song_url).send().await {
                    if resp.ok() {
                        if let Ok(ex) = resp.json::<SongExampleFile>().await {
                            on_code_change.emit(ex.code.clone());
                            if let Ok(p) = Pattern::parse(&ex.code) {
                                on_pattern_change.emit(p);
                            }
                            if let Some(cb) = on_bpm_change.as_ref() {
                                if let Some(bpm) = ex.bpm {
                                    cb.emit(bpm);
                                }
                            }
                            song_details.set(ex.into());
                        }
                    }
                }
            });
        })
    };

    let oninput = {
        let on_code_change = props.on_code_change.clone();
        let on_pattern_change = props.on_pattern_change.clone();
        Callback::from(move |e: InputEvent| {
            let input: web_sys::HtmlInputElement = e.target_unchecked_into();
            let value = input.value();
            on_code_change.emit(value.clone());
            if let Ok(p) = Pattern::parse(&value) {
                on_pattern_change.emit(p);
            }
        })
    };

    let on_preset_change = {
        let on_code_change = props.on_code_change.clone();
        let on_pattern_change = props.on_pattern_change.clone();
        let presets = props.presets.clone();
        Callback::from(move |e: Event| {
            let select: web_sys::HtmlSelectElement = e.target_unchecked_into();
            let idx = select.selected_index() as i32;
            if idx >= 1 {
                let idx = idx as usize;
                if idx <= presets.len() {
                    let code = presets[idx - 1].code.clone();
                    on_code_change.emit(code.clone());
                    if let Ok(p) = Pattern::parse(&code) {
                        on_pattern_change.emit(p);
                    }
                }
            }
        })
    };

    let on_save = {
        let code = props.code.clone();
        let bpm = props.bpm;
        Callback::from(move |_| {
            let _ = export::export_json(&code, bpm);
        })
    };

    let on_export_midi = {
        let pattern = props.pattern.clone();
        let bpm = props.bpm;
        Callback::from(move |_| {
            let _ = export::export_midi(&pattern, bpm);
        })
    };

    let on_load_click = {
        let file_input_ref = file_input_ref.clone();
        Callback::from(move |_| {
            if let Some(input) = file_input_ref.cast::<web_sys::HtmlInputElement>() {
                input.click();
            }
        })
    };

    const DEFAULT_BPM: u32 = 120;
    let on_new_song = {
        let on_code_change = props.on_code_change.clone();
        let on_pattern_change = props.on_pattern_change.clone();
        let on_bpm_change = props.on_bpm_change.clone();
        let song_details = song_details.clone();
        Callback::from(move |_: MouseEvent| {
            on_code_change.emit(String::new());
            on_pattern_change.emit(Pattern::new());
            if let Some(cb) = on_bpm_change.as_ref() {
                cb.emit(DEFAULT_BPM);
            }
            song_details.set(SongInfo::default());
        })
    };

    let on_file_change = {
        let on_code_change = props.on_code_change.clone();
        let on_pattern_change = props.on_pattern_change.clone();
        let on_bpm_change = props.on_bpm_change.clone();
        let song_details = song_details.clone();
        Callback::from(move |e: Event| {
            let input: web_sys::HtmlInputElement = e.target_unchecked_into();
            if let Some(file) = input.files().and_then(|f| f.get(0)) {
                let reader = web_sys::FileReader::new().unwrap();
                let on_code_change = on_code_change.clone();
                let on_pattern_change = on_pattern_change.clone();
                let on_bpm_change = on_bpm_change.clone();
                let song_details = song_details.clone();
                let onload = wasm_bindgen::closure::Closure::wrap(Box::new(move |e: Event| {
                    let r: web_sys::FileReader = e.target_unchecked_into();
                    if let Ok(Some(jsv)) = r.result().map(|v| v.as_string()) {
                        // 1) Session format (.json): code + bpm only; no song metadata
                        if let Ok(session) = serde_json::from_str::<crate::export::SessionFile>(&jsv) {
                            on_code_change.emit(session.code.clone());
                            if let Ok(p) = Pattern::parse(&session.code) {
                                on_pattern_change.emit(p);
                            }
                            if let Some(cb) = on_bpm_change.as_ref() {
                                cb.emit(session.bpm);
                            }
                            song_details.set(SongInfo::default());
                            return;
                        }
                        // 2) Song format (.jsong): code + metadata; show code and editable details
                        if let Ok(song) = serde_json::from_str::<SongExampleFile>(&jsv) {
                            on_code_change.emit(song.code.clone());
                            if let Ok(p) = Pattern::parse(&song.code) {
                                on_pattern_change.emit(p);
                            }
                            if let Some(cb) = on_bpm_change.as_ref() {
                                if let Some(bpm) = song.bpm {
                                    cb.emit(bpm);
                                }
                            }
                            song_details.set(song.into());
                            return;
                        }
                        // 3) Unknown JSON or plain text: try parsing as pattern only if it looks like code
                        let trimmed = jsv.trim();
                        if !trimmed.starts_with('{') && !trimmed.starts_with('[') {
                            if let Ok(p) = Pattern::parse(trimmed) {
                                on_code_change.emit(trimmed.to_string());
                                on_pattern_change.emit(p);
                            }
                        }
                        song_details.set(SongInfo::default());
                    }
                }) as Box<dyn FnMut(Event)>);
                reader.set_onload(Some(onload.as_ref().unchecked_ref()));
                onload.forget();
                reader.read_as_text(&file).unwrap();
            }
        })
    };

    let on_song_title = {
        let song_details = song_details.clone();
        Callback::from(move |e: InputEvent| {
            let input: web_sys::HtmlInputElement = e.target_unchecked_into();
            let mut info = (*song_details).clone();
            info.title = opt_value(input.value());
            song_details.set(info);
        })
    };
    let on_song_bpm = {
        let song_details = song_details.clone();
        let on_bpm_change = props.on_bpm_change.clone();
        Callback::from(move |e: InputEvent| {
            let input: web_sys::HtmlInputElement = e.target_unchecked_into();
            let val = input.value();
            let bpm = val.trim().parse().ok();
            if let Some(b) = bpm {
                if let Some(cb) = on_bpm_change.as_ref() {
                    cb.emit(b);
                }
            }
            let mut info = (*song_details).clone();
            info.bpm = bpm;
            song_details.set(info);
        })
    };
    let on_song_composer = {
        let song_details = song_details.clone();
        Callback::from(move |e: InputEvent| {
            let input: web_sys::HtmlInputElement = e.target_unchecked_into();
            let mut info = (*song_details).clone();
            info.composer = opt_value(input.value());
            song_details.set(info);
        })
    };
    let on_song_description = {
        let song_details = song_details.clone();
        Callback::from(move |e: InputEvent| {
            let input: web_sys::HtmlTextAreaElement = e.target_unchecked_into();
            let mut info = (*song_details).clone();
            info.description = opt_value(input.value());
            song_details.set(info);
        })
    };
    let on_song_link = {
        let song_details = song_details.clone();
        Callback::from(move |e: InputEvent| {
            let input: web_sys::HtmlInputElement = e.target_unchecked_into();
            let mut info = (*song_details).clone();
            info.link = opt_value(input.value());
            song_details.set(info);
        })
    };
    let presets = &props.presets;
    let details = (*song_details).clone();

    html! {
        <div class="editor-panel panel">
            <div class="editor-toolbar">
                <span class="toolbar-label">{"Presets:"}</span>
                <select class="select-preset" onchange={on_preset_change}>
                    <option value="">{"Choose a pattern…"}</option>
                    {for presets.iter().map(|p| {
                        html! { <option value={p.name.clone()}>{&p.name}</option> }
                    })}
                </select>
                {if !(*examples).is_empty() {
                    html! {
                        <select class="select-preset" onchange={on_example_change}>
                            <option value="">{"Select song"}</option>
                            {for (*examples).iter().map(|ex| {
                                html! { <option value={ex.id.clone()}>{&ex.title}</option> }
                            })}
                        </select>
                    }
                } else {
                    html! {}
                }}
                <div class="toolbar-actions">
                    <button type="button" class="btn-tool btn-tool-pro" onclick={on_new_song} title="New song">
                        <span class="btn-tool-icon" aria-hidden="true">{"⊕"}</span>
                        <span>{"New"}</span>
                    </button>
                    <button type="button" class="btn-tool btn-tool-pro" onclick={on_load_click} title="Load file">
                        <span class="btn-tool-icon" aria-hidden="true">{"📂"}</span>
                        <span>{"Load"}</span>
                    </button>
                    <button type="button" class="btn-tool btn-tool-pro" onclick={on_save} title="Save session">
                        <span class="btn-tool-icon" aria-hidden="true">{"💾"}</span>
                        <span>{"Save"}</span>
                    </button>
                    <button type="button" class="btn-tool btn-tool-pro" onclick={on_export_midi} title="Export as MIDI">
                        <span class="btn-tool-icon" aria-hidden="true">{"⬇"}</span>
                        <span>{"Export"}</span>
                    </button>
                </div>
            </div>
            <input
                ref={file_input_ref}
                type="file"
                accept=".jsong,.json,application/json"
                class="hidden-file-input"
                onchange={on_file_change}
            />
            <details class="song-details" open={true}>
                <summary>
                    <span class="song-details-summary-label">{"Song details"}</span>
                    {if let Some(t) = details.title.as_deref().filter(|s| !s.trim().is_empty()) {
                        html! { <span class="song-details-summary-title">{ " — " }{ t }</span> }
                    } else {
                        html! {}
                    }}
                </summary>
                <div class="song-details-fields">
                    <label>{"Title"}
                        <input
                            type="text"
                            class="song-detail-input"
                            placeholder="Song title"
                            value={details.title.as_deref().unwrap_or("").to_string()}
                            oninput={on_song_title.clone()}
                        />
                    </label>
                    <label>{"BPM"}
                        <input
                            type="number"
                            class="song-detail-input"
                            placeholder="120"
                            min="1"
                            max="999"
                            value={details.bpm.or(Some(props.bpm)).map(|b| b.to_string()).unwrap_or_else(|| String::new())}
                            oninput={on_song_bpm.clone()}
                        />
                    </label>
                    <label>{"Composer"}
                        <input
                            type="text"
                            class="song-detail-input"
                            placeholder="Composer"
                            value={details.composer.as_deref().unwrap_or("").to_string()}
                            oninput={on_song_composer.clone()}
                        />
                    </label>
                    <label>{"Description"}
                        <textarea
                            class="song-detail-textarea"
                            placeholder="Description"
                            rows="2"
                            value={details.description.as_deref().unwrap_or("").to_string()}
                            oninput={on_song_description.clone()}
                        />
                    </label>
                    <label>{"Link"}
                        <input
                            type="url"
                            class="song-detail-input"
                            placeholder="https://..."
                            value={details.link.as_deref().unwrap_or("").to_string()}
                            oninput={on_song_link.clone()}
                        />
                    </label>
                </div>
            </details>
            <div class="code-input-wrap">
                <div class="code-highlight-layer" aria-hidden="true">
                    {{
                        let code = props.code.clone();
                        let comment_ranges = get_comment_ranges(&code);
                        let play_ranges: Vec<(usize, usize)> = props.pattern.events
                            .iter()
                            .enumerate()
                            .filter(|(i, _)| props.current_playing_indices.contains(i))
                            .filter_map(|(_, e)| e.span)
                            .collect();
                        let segments = build_code_layer_segments(&code, &comment_ranges, &play_ranges);
                        let code2 = code.clone();
                        let layer_items: Html = segments.into_iter().map(move |(s, e, is_comment, is_playing)| {
                            let content = escape_html(&code2[s..e]);
                            let mut class = String::new();
                            if is_comment {
                                class.push_str("code-comment");
                            }
                            if is_playing {
                                if !class.is_empty() {
                                    class.push(' ');
                                }
                                class.push_str("code-play-highlight");
                            }
                            html! { <span class={class}>{ content }</span> }
                        }).collect::<Html>();
                        { layer_items }
                    }}
                </div>
                <textarea
                    class="code-input code-input-overlay"
                    placeholder="One line = one timeline. Or use channels: @4 bd sd bd sd then @16 A4 C5 E5 G5 ... (next line = track, @N = steps/bar)"
                    oninput={oninput}
                    value={props.code.clone()}
                />
            </div>
            <footer class="editor-footer">
                {
                    {
                        let p = &props.pattern;
                        let bpm = props.bpm as f32;
                        let beats = p.duration_16ths() / 4.0;
                        let sec = if bpm > 0.0 { beats * 60.0 / bpm } else { 0.0 };
                        let notes = p.events.len();
                        let mut keys: Vec<&str> = p.events.iter().map(|e| e.sample.as_str()).collect();
                        keys.sort_unstable();
                        keys.dedup();
                        let keys_display: String = if keys.is_empty() { "—".into() } else { keys.join(", ") };
                        html! {
                            <>
                                <span class="editor-footer-item">{"Duration: "}{format!("{:.1}s", sec)}{" ("}{format!("{:.1}", beats)}{" beats)"}</span>
                                <span class="editor-footer-item">{"Notes: "}{notes}</span>
                                <span class="editor-footer-item">{"Samples/notes: "}{keys_display}</span>
                            </>
                        }
                    }
                }
            </footer>
        </div>
    }
}
