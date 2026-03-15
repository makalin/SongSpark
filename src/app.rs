use yew::prelude::*;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;
use gloo_net::http::Request;
use gloo_timers::callback::Interval;
use crate::components::{About, Editor, Metronome, Mixer, Player, QuickInfo, SampleLibrary, Settings, StepStrip};
use crate::patterns::Pattern;
use crate::audio::AudioEngine;
use crate::presets::{Preset, default_presets};

#[function_component(App)]
pub fn app() -> Html {
    let current_pattern = use_state(Pattern::new);
    let code = use_state(String::new);
    let audio_engine = use_state(|| AudioEngine::new().unwrap());
    let bpm = use_state(|| 120u32);
    let theme = use_state(|| "dark".to_string());
    let show_settings = use_state(|| false);
    let show_about = use_state(|| false);
    let is_playing = use_state(|| false);
    let play_interval = use_state(|| Option::<Interval>::None);
    let play_start_time = use_state(|| Option::<f64>::None);
    let current_playing_indices = use_state(|| Vec::<usize>::new());
    let presets = use_state(|| default_presets());
    let pack_names = use_state(|| Vec::<String>::new());
    let active_pack = use_state(|| String::new());
    let drums_volume = use_state(|| 1.0f32);
    let drums_pan = use_state(|| 0.0f32);
    let synth_volume = use_state(|| 1.0f32);
    let synth_pan = use_state(|| 0.0f32);
    let preview_volume = use_state(|| 1.0f32);
    let preview_pan = use_state(|| 0.0f32);

    // Fetch presets from file
    let presets_handle = presets.clone();
    use_effect_with_deps(
        move |_| {
            let h = presets_handle.clone();
            wasm_bindgen_futures::spawn_local(async move {
                if let Ok(resp) = Request::get("examples/presets.json").send().await {
                    if resp.ok() {
                        if let Ok(loaded) = resp.json::<Vec<Preset>>().await {
                            h.set(loaded);
                        }
                    }
                }
            });
            || ()
        },
        (),
    );

    // Spacebar: same as Play/Stop button (must run after on_toggle_play is defined)

    // Apply theme to body
    use_effect_with_deps(
        move |t| {
            if let Some(doc) = web_sys::window().and_then(|w| w.document()) {
                let _ = doc.body().map(|b| b.set_attribute("data-theme", t).ok());
            }
            || ()
        },
        (*theme).clone(),
    );

    let current_pattern_handle = current_pattern.clone();
    let on_pattern_change = Callback::from(move |p: Pattern| current_pattern_handle.set(p));
    let on_code_change = {
        let code = code.clone();
        let current_pattern = current_pattern.clone();
        Callback::from(move |s: String| {
            code.set(s.clone());
            if let Ok(p) = Pattern::parse(&s) {
                current_pattern.set(p);
            }
        })
    };

    let on_toggle_play = {
        let is_playing = is_playing.clone();
        let play_interval = play_interval.clone();
        let current_pattern = current_pattern.clone();
        let audio_engine = audio_engine.clone();
        let bpm = bpm.clone();
        let drums_volume = drums_volume.clone();
        let drums_pan = drums_pan.clone();
        let synth_volume = synth_volume.clone();
        let synth_pan = synth_pan.clone();
        let play_start_time_toggle = play_start_time.clone();
        let current_playing_indices_toggle = current_playing_indices.clone();
        Callback::from(move |_| {
            if *is_playing {
                play_interval.set(None);
                is_playing.set(false);
                play_start_time_toggle.set(None);
                current_playing_indices_toggle.set(vec![]);
                return;
            }
            let pattern = (*current_pattern).clone();
            if pattern.events.is_empty() {
                return;
            }
            let bpm_f = *bpm as f64;
            let step_sec = 60.0 / (bpm_f * 4.0);
            let pattern_duration_sec = pattern.duration_16ths() as f64 * step_sec;
            let pattern_to_sec = 4.0 * step_sec;
            let step_sec_f64 = step_sec;
            let eng = (*audio_engine).clone();
            eng.resume();
            let now = eng.current_time();
            play_start_time_toggle.set(Some(now));
            let drums_vol = *drums_volume;
            let drums_p = *drums_pan;
            let synth_vol = *synth_volume;
            let synth_p = *synth_pan;
            for event in &pattern.events {
                let when = now + (event.time as f64) * pattern_to_sec;
                if event.is_note {
                    let _ = eng.play_synth_note_at(&event.sample, pattern.gain * synth_vol, synth_p, when, step_sec_f64);
                } else {
                    let _ = eng.play_sample_at(&event.sample, pattern.gain * drums_vol, drums_p, when);
                }
            }

            let audio_engine = audio_engine.clone();
            let pattern = pattern.clone();
            let bpm_val = *bpm;
            let drums_volume = drums_volume.clone();
            let drums_pan = drums_pan.clone();
            let synth_volume = synth_volume.clone();
            let synth_pan = synth_pan.clone();
            let pattern_duration_ms = (pattern_duration_sec * 1000.0).max(100.0) as u32;
            let interval = Interval::new(pattern_duration_ms, move || {
                if pattern.events.is_empty() {
                    return;
                }
                let step_sec = 60.0 / (bpm_val as f64 * 4.0);
                let pattern_to_sec = 4.0 * step_sec;
                let eng = (*audio_engine).clone();
                let now = eng.current_time();
                for event in &pattern.events {
                    let when = now + (event.time as f64) * pattern_to_sec;
                    if event.is_note {
                        let _ = eng.play_synth_note_at(&event.sample, pattern.gain * (*synth_volume), *synth_pan, when, step_sec);
                    } else {
                        let _ = eng.play_sample_at(&event.sample, pattern.gain * (*drums_volume), *drums_pan, when);
                    }
                }
            });
            play_interval.set(Some(interval));
            is_playing.set(true);
        })
    };

    // When pattern or BPM changes while playing, replace the interval so the loop uses the new code
    // Highlight currently playing notes: update current_playing_indices every 40ms when playing
    let play_highlight_interval = use_state(|| Option::<Interval>::None);
    let audio_engine_hl = audio_engine.clone();
    let play_start_time_hl = play_start_time.clone();
    let current_pattern_hl = current_pattern.clone();
    let current_playing_indices_set = current_playing_indices.clone();
    let bpm_hl = bpm.clone();
    use_effect_with_deps(
        move |(playing, start_opt, pat, b)| {
            if !*playing {
                current_playing_indices_set.set(vec![]);
                play_highlight_interval.set(None);
                return;
            }
            if start_opt.is_none() || pat.events.is_empty() {
                return;
            }
            let pattern = pat.clone();
            let step_sec = 60.0 / (*b as f64 * 4.0);
            let pattern_to_sec = 4.0 * step_sec; // same as audio: event.time is in beats, * pattern_to_sec = seconds
            let pattern_duration_sec = pattern.duration_16ths() as f64 * step_sec;
            let pattern_duration_sec = pattern_duration_sec.max(0.01);
            // Highlight window: show active note for ~50ms around exact hit so it stays in sync with sound
            let highlight_window_sec = 0.05;
            let interval = Interval::new(40, move || {
                let eng = (*audio_engine_hl).clone();
                let now = eng.current_time();
                let start = (*play_start_time_hl).unwrap_or(0.0);
                let elapsed = (now - start).rem_euclid(pattern_duration_sec);
                let mut indices = Vec::new();
                for (i, event) in pattern.events.iter().enumerate() {
                    let event_t_sec = event.time as f64 * pattern_to_sec;
                    if (elapsed - event_t_sec).abs() < highlight_window_sec {
                        indices.push(i);
                    }
                }
                current_playing_indices_set.set(indices);
            });
            play_highlight_interval.set(Some(interval));
        },
        (*is_playing, *play_start_time, (*current_pattern).clone(), *bpm),
    );

    let play_interval_sync = play_interval.clone();
    let play_start_time_sync = play_start_time.clone();
    let current_pattern_sync = current_pattern.clone();
    let audio_engine_sync = audio_engine.clone();
    let bpm_sync = bpm.clone();
    let is_playing_sync = is_playing.clone();
    let drums_volume_sync = drums_volume.clone();
    let drums_pan_sync = drums_pan.clone();
    let synth_volume_sync = synth_volume.clone();
    let synth_pan_sync = synth_pan.clone();
    // Run only when pattern or BPM changes (not on initial play) so we don't double-schedule
    use_effect_with_deps(
        move |(pat, b)| {
            if !*is_playing_sync {
                return;
            }
            let pattern = pat.clone();
            if pattern.events.is_empty() {
                return;
            }
            let bpm_f = *b as f64;
            let step_sec = 60.0 / (bpm_f * 4.0);
            let pattern_to_sec = 4.0 * step_sec;
            let pattern_duration_sec = pattern.duration_16ths() as f64 * step_sec;
            let pattern_duration_ms = (pattern_duration_sec * 1000.0).max(100.0) as u32;

            play_interval_sync.set(None);
            let eng = (*audio_engine_sync).clone();
            let now = eng.current_time();
            play_start_time_sync.set(Some(now));
            let step_sec = 60.0 / (*b as f64 * 4.0);
            for event in &pattern.events {
                let when = now + (event.time as f64) * pattern_to_sec;
                if event.is_note {
                    let _ = eng.play_synth_note_at(&event.sample, pattern.gain * (*synth_volume_sync), *synth_pan_sync, when, step_sec);
                } else {
                    let _ = eng.play_sample_at(&event.sample, pattern.gain * (*drums_volume_sync), *drums_pan_sync, when);
                }
            }
            let audio_engine = audio_engine_sync.clone();
            let pattern = pattern.clone();
            let bpm_val = *b;
            let drums_volume = drums_volume_sync.clone();
            let drums_pan = drums_pan_sync.clone();
            let synth_volume = synth_volume_sync.clone();
            let synth_pan = synth_pan_sync.clone();
            let interval = Interval::new(pattern_duration_ms, move || {
                if pattern.events.is_empty() {
                    return;
                }
                let step_sec = 60.0 / (bpm_val as f64 * 4.0);
                let pt = 4.0 * step_sec;
                let eng = (*audio_engine).clone();
                let now = eng.current_time();
                for event in &pattern.events {
                    let when = now + (event.time as f64) * pt;
                    if event.is_note {
                        let _ = eng.play_synth_note_at(&event.sample, pattern.gain * (*synth_volume), *synth_pan, when, step_sec);
                    } else {
                        let _ = eng.play_sample_at(&event.sample, pattern.gain * (*drums_volume), *drums_pan, when);
                    }
                }
            });
            play_interval_sync.set(Some(interval));
        },
        ((*current_pattern_sync).clone(), *bpm_sync),
    );

    let on_toggle_play_for_key = on_toggle_play.clone();
    use_effect_with_deps(
        move |_| {
            let cb = on_toggle_play_for_key.clone();
            let window = web_sys::window().unwrap();
            let closure = Closure::wrap(Box::new(move |e: web_sys::KeyboardEvent| {
                if e.key() != " " {
                    return;
                }
                // Don't steal Space when typing in inputs (so user can type spaces in the pattern)
                if let Some(target) = e.target() {
                    if let Some(el) = target.dyn_ref::<web_sys::HtmlElement>() {
                        let tag = el.tag_name().to_uppercase();
                        if tag == "INPUT" || tag == "TEXTAREA" || tag == "SELECT" {
                            return;
                        }
                    }
                }
                e.prevent_default();
                cb.emit(());
            }) as Box<dyn FnMut(web_sys::KeyboardEvent)>);
            let _ = window.add_event_listener_with_callback("keydown", closure.as_ref().unchecked_ref());
            closure.forget();
            || ()
        },
        (),
    );

    let on_pack_added = {
        let pack_names = pack_names.clone();
        let active_pack = active_pack.clone();
        Callback::from(move |name: String| {
            let mut names = (*pack_names).clone();
            if !names.contains(&name) {
                names.push(name.clone());
                pack_names.set(names);
            }
            active_pack.set(name);
        })
    };

    let active_pack_handle = active_pack.clone();
    let on_active_pack_change = Callback::from(move |name: String| active_pack_handle.set(name));

    let bpm_handle = bpm.clone();
    let theme_handle = theme.clone();
    let open_handle = show_settings.clone();
    let close_handle = show_settings.clone();
    let show_about_open = show_about.clone();
    let show_about_close = show_about.clone();
    let on_bpm_change = Callback::from(move |v: u32| bpm_handle.set(v));
    let on_theme_change = Callback::from(move |t: String| theme_handle.set(t));
    let on_open_settings = Callback::from(move |_| open_handle.set(true));
    let on_close_settings = Callback::from(move |_| close_handle.set(false));
    let on_open_about = Callback::from(move |_| show_about_open.set(true));
    let on_close_about = Callback::from(move |_| show_about_close.set(false));
    let on_drums_volume = Callback::from({
        let h = drums_volume.clone();
        move |v: f32| h.set(v)
    });
    let on_drums_pan = Callback::from({
        let h = drums_pan.clone();
        move |v: f32| h.set(v)
    });
    let on_preview_volume = Callback::from({
        let h = preview_volume.clone();
        move |v: f32| h.set(v)
    });
    let on_preview_pan = Callback::from({
        let h = preview_pan.clone();
        move |v: f32| h.set(v)
    });
    let on_synth_volume = Callback::from({
        let h = synth_volume.clone();
        move |v: f32| h.set(v)
    });
    let on_synth_pan = Callback::from({
        let h = synth_pan.clone();
        move |v: f32| h.set(v)
    });

    let on_toolbar_bpm_change = {
        let on_bpm_change = on_bpm_change.clone();
        Callback::from(move |e: Event| {
            let input = e.target_dyn_into::<web_sys::HtmlInputElement>();
            if let Some(inp) = input {
                if let Ok(v) = inp.value().trim().parse::<u32>() {
                    if (1..=999).contains(&v) {
                        on_bpm_change.emit(v);
                    }
                }
            }
        })
    };

    html! {
        <div class="app">
            <header class="header">
                <h1 class="logo">{"SONGSPARK"}</h1>
                <p class="tagline">{"LIVE CODING • WASM"}</p>
                <div class="toolbar">
                    <div class="toolbar-tempo">
                        <label class="toolbar-tempo-label" for="toolbar-bpm">{"BPM"}</label>
                        <input
                            id="toolbar-bpm"
                            type="number"
                            class="toolbar-tempo-input"
                            min="1"
                            max="999"
                            value={(*bpm).to_string()}
                            onchange={on_toolbar_bpm_change}
                        />
                    </div>
                    <Metronome bpm={*bpm} />
                    <Player
                        pattern={Some((*current_pattern).clone())}
                        audio_engine={audio_engine.clone()}
                        is_playing={*is_playing}
                        on_toggle={on_toggle_play.clone()}
                    />
                    <a href="https://github.com/frangedev/SongSpark" target="_blank" rel="noopener noreferrer" class="btn-link">{"GitHub"}</a>
                    <button class="btn-icon" onclick={on_open_about} title="About">{"ℹ"}</button>
                    <button class="btn-icon" onclick={on_open_settings} title="Settings">{"⚙"}</button>
                </div>
            </header>
            <main class="main">
                <section class="how-to-panel">
                    <details>
                        <summary>{"How to play"}</summary>
                        <ol class="how-to-steps">
                            <li>{"Banks: add packs or load from URL (Strudel-style names: bd, sd, hh, cp, rim, oh, cr, rd). Switch pack = switch sound."}</li>
                            <li>{"Pattern: preset dropdown or type "}<code>{"bd sd hh*2 cp"}</code>{" ("}<code>{"*n"}</code>{" = repeat). Play / Spacebar = play/stop (Space works normally when typing in the pattern box)."}</li>
                            <li>{"Live coding: change the pattern or BPM while playing — the loop updates on the next cycle."}</li>
                            <li>{"Music notation: currently drum/sample names (bd, sd, hh…). Pitched notes (e.g. mini-notation) are planned."}</li>
                        </ol>
                    </details>
                </section>
                <div class="workspace">
                    <Editor
                        code={(*code).clone()}
                        on_code_change={on_code_change.clone()}
                        on_pattern_change={on_pattern_change}
                        pattern={(*current_pattern).clone()}
                        bpm={*bpm}
                        on_bpm_change={Some(on_bpm_change.clone())}
                        presets={(*presets).clone()}
                        current_playing_indices={(*current_playing_indices).clone()}
                    />
                    <div class="right-panel">
                        <SampleLibrary
                            audio_engine={audio_engine.clone()}
                            pack_names={(*pack_names).clone()}
                            active_pack={(*active_pack).clone()}
                            on_active_pack_change={on_active_pack_change}
                            on_pack_added={on_pack_added}
                            preview_volume={*preview_volume}
                            preview_pan={*preview_pan}
                        />
                        <StepStrip is_playing={*is_playing} bpm={*bpm} />
                        <Mixer
                            audio_engine={audio_engine.clone()}
                            drums_volume={*drums_volume}
                            drums_pan={*drums_pan}
                            synth_volume={*synth_volume}
                            synth_pan={*synth_pan}
                            preview_volume={*preview_volume}
                            preview_pan={*preview_pan}
                            on_drums_volume={on_drums_volume.clone()}
                            on_drums_pan={on_drums_pan.clone()}
                            on_synth_volume={on_synth_volume.clone()}
                            on_synth_pan={on_synth_pan.clone()}
                            on_preview_volume={on_preview_volume.clone()}
                            on_preview_pan={on_preview_pan.clone()}
                        />
                        <QuickInfo
                            bpm={*bpm}
                            pattern={(*current_pattern).clone()}
                        />
                    </div>
                </div>
            </main>
            {if *show_settings {
                html! {
                    <Settings
                        bpm={*bpm}
                        on_bpm_change={on_bpm_change}
                        theme={(*theme).clone()}
                        on_theme_change={on_theme_change}
                        on_close={on_close_settings}
                    />
                }
            } else {
                html! {}
            }}
            {if *show_about {
                html! { <About on_close={on_close_about} /> }
            } else {
                html! {}
            }}
        </div>
    }
}
