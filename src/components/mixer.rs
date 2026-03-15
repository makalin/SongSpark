use yew::prelude::*;
use crate::audio::AudioEngine;

#[derive(Properties, PartialEq)]
pub struct MixerProps {
    pub audio_engine: UseStateHandle<AudioEngine>,
    pub drums_volume: f32,
    pub drums_pan: f32,
    pub synth_volume: f32,
    pub synth_pan: f32,
    pub preview_volume: f32,
    pub preview_pan: f32,
    pub on_drums_volume: Callback<f32>,
    pub on_drums_pan: Callback<f32>,
    pub on_synth_volume: Callback<f32>,
    pub on_synth_pan: Callback<f32>,
    pub on_preview_volume: Callback<f32>,
    pub on_preview_pan: Callback<f32>,
}

fn vol_to_pct(v: f32) -> u32 {
    (v.clamp(0.0, 2.0) * 100.0) as u32
}

fn pct_to_vol(p: f32) -> f32 {
    (p / 100.0).clamp(0.0, 2.0)
}

/// Pan -1..1 to slider 0..100 (L..R)
fn pan_to_slider(p: f32) -> u32 {
    ((p.clamp(-1.0, 1.0) + 1.0) * 50.0).round() as u32
}

/// Slider 0..100 to pan -1..1
fn slider_to_pan(s: f32) -> f32 {
    (s / 50.0 - 1.0).clamp(-1.0, 1.0)
}

#[function_component(Mixer)]
pub fn mixer(props: &MixerProps) -> Html {
    let master_volume = use_state(|| 1.0f32);

    let on_master_input = {
        let audio_engine = props.audio_engine.clone();
        let master_volume = master_volume.clone();
        Callback::from(move |e: InputEvent| {
            let input: web_sys::HtmlInputElement = e.target_unchecked_into();
            if let Ok(v) = input.value().trim().parse::<f32>() {
                let v = pct_to_vol(v);
                master_volume.set(v);
                (*audio_engine).set_master_volume(v);
            }
        })
    };

    let on_drums_vol = {
        let cb = props.on_drums_volume.clone();
        Callback::from(move |e: InputEvent| {
            let input: web_sys::HtmlInputElement = e.target_unchecked_into();
            if let Ok(v) = input.value().trim().parse::<f32>() {
                cb.emit(pct_to_vol(v));
            }
        })
    };
    let on_drums_pan = {
        let cb = props.on_drums_pan.clone();
        Callback::from(move |e: InputEvent| {
            let input: web_sys::HtmlInputElement = e.target_unchecked_into();
            if let Ok(v) = input.value().trim().parse::<f32>() {
                cb.emit(slider_to_pan(v));
            }
        })
    };
    let on_synth_vol = {
        let cb = props.on_synth_volume.clone();
        Callback::from(move |e: InputEvent| {
            let input: web_sys::HtmlInputElement = e.target_unchecked_into();
            if let Ok(v) = input.value().trim().parse::<f32>() {
                cb.emit(pct_to_vol(v));
            }
        })
    };
    let on_synth_pan = {
        let cb = props.on_synth_pan.clone();
        Callback::from(move |e: InputEvent| {
            let input: web_sys::HtmlInputElement = e.target_unchecked_into();
            if let Ok(v) = input.value().trim().parse::<f32>() {
                cb.emit(slider_to_pan(v));
            }
        })
    };
    let on_preview_vol = {
        let cb = props.on_preview_volume.clone();
        Callback::from(move |e: InputEvent| {
            let input: web_sys::HtmlInputElement = e.target_unchecked_into();
            if let Ok(v) = input.value().trim().parse::<f32>() {
                cb.emit(pct_to_vol(v));
            }
        })
    };
    let on_preview_pan = {
        let cb = props.on_preview_pan.clone();
        Callback::from(move |e: InputEvent| {
            let input: web_sys::HtmlInputElement = e.target_unchecked_into();
            if let Ok(v) = input.value().trim().parse::<f32>() {
                cb.emit(slider_to_pan(v));
            }
        })
    };

    html! {
        <div class="mixer panel">
            <h3 class="panel-title">{"Mixer"}</h3>
            <div class="mixer-racks">
                <div class="mixer-channel mixer-channel-equal">
                    <span class="mixer-channel-name">{"Master"}</span>
                    <div class="mixer-fader-vertical">
                        <input
                            type="range"
                            class="mixer-fader"
                            min="0"
                            max="200"
                            value={vol_to_pct(*master_volume).to_string()}
                            oninput={on_master_input}
                        />
                    </div>
                    <span class="mixer-fader-value">{format!("{:.0}%", (*master_volume) * 100.0)}</span>
                    <div class="mixer-pan mixer-pan-dummy" aria-hidden="true">
                        <span class="mixer-pan-label">{"L"}</span>
                        <span class="mixer-pan-fill" />
                        <span class="mixer-pan-label">{"R"}</span>
                    </div>
                    <div class="mixer-fx-slot">
                        <span class="mixer-fx-label">{"FX"}</span>
                        <select class="mixer-fx-select" disabled={true}>
                            <option value="none">{"—"}</option>
                        </select>
                    </div>
                </div>
                <div class="mixer-channel mixer-channel-equal">
                    <span class="mixer-channel-name">{"Drums"}</span>
                    <div class="mixer-fader-vertical">
                        <input type="range" class="mixer-fader" min="0" max="200" value={vol_to_pct(props.drums_volume).to_string()} oninput={on_drums_vol} />
                    </div>
                    <span class="mixer-fader-value">{format!("{:.0}%", props.drums_volume * 100.0)}</span>
                    <div class="mixer-pan">
                        <span class="mixer-pan-label">{"L"}</span>
                        <input type="range" class="mixer-pan-slider" min="0" max="100" step="1" value={pan_to_slider(props.drums_pan).to_string()} oninput={on_drums_pan} />
                        <span class="mixer-pan-label">{"R"}</span>
                    </div>
                    <div class="mixer-fx-slot">
                        <span class="mixer-fx-label">{"FX"}</span>
                        <select class="mixer-fx-select" disabled={true}><option value="none">{"—"}</option></select>
                    </div>
                </div>
                <div class="mixer-channel mixer-channel-equal">
                    <span class="mixer-channel-name">{"Synth"}</span>
                    <div class="mixer-fader-vertical">
                        <input type="range" class="mixer-fader" min="0" max="200" value={vol_to_pct(props.synth_volume).to_string()} oninput={on_synth_vol} />
                    </div>
                    <span class="mixer-fader-value">{format!("{:.0}%", props.synth_volume * 100.0)}</span>
                    <div class="mixer-pan">
                        <span class="mixer-pan-label">{"L"}</span>
                        <input type="range" class="mixer-pan-slider" min="0" max="100" step="1" value={pan_to_slider(props.synth_pan).to_string()} oninput={on_synth_pan} />
                        <span class="mixer-pan-label">{"R"}</span>
                    </div>
                    <div class="mixer-fx-slot">
                        <span class="mixer-fx-label">{"FX"}</span>
                        <select class="mixer-fx-select" disabled={true}><option value="none">{"—"}</option></select>
                    </div>
                </div>
                <div class="mixer-channel mixer-channel-equal">
                    <span class="mixer-channel-name">{"Preview"}</span>
                    <div class="mixer-fader-vertical">
                        <input type="range" class="mixer-fader" min="0" max="200" value={vol_to_pct(props.preview_volume).to_string()} oninput={on_preview_vol} />
                    </div>
                    <span class="mixer-fader-value">{format!("{:.0}%", props.preview_volume * 100.0)}</span>
                    <div class="mixer-pan">
                        <span class="mixer-pan-label">{"L"}</span>
                        <input type="range" class="mixer-pan-slider" min="0" max="100" step="1" value={pan_to_slider(props.preview_pan).to_string()} oninput={on_preview_pan} />
                        <span class="mixer-pan-label">{"R"}</span>
                    </div>
                    <div class="mixer-fx-slot">
                        <span class="mixer-fx-label">{"FX"}</span>
                        <select class="mixer-fx-select" disabled={true}><option value="none">{"—"}</option></select>
                    </div>
                </div>
            </div>
        </div>
    }
}
