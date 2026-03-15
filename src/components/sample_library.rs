use yew::prelude::*;
use web_sys::{FileReader, HtmlInputElement, HtmlSelectElement, AudioBuffer};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use gloo_net::http::Request;
use crate::audio::AudioEngine;
use std::collections::HashMap;
use serde::Deserialize;

#[derive(Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PackMetadata {
    #[serde(default)]
    pub composer: Option<String>,
    #[serde(default)]
    pub album_cover_url: Option<String>,
    #[serde(default)]
    pub link: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Clone, Deserialize)]
struct BankFile {
    #[serde(rename = "bankName")]
    bank_name: String,
    #[serde(rename = "baseUrl")]
    base_url: String,
    #[serde(default)]
    metadata: Option<PackMetadata>,
    samples: HashMap<String, String>,
}

/// Embedded example pack so we don't depend on fetching packs/example.json.
fn embedded_example_bank() -> BankFile {
    let mut samples = HashMap::new();
    samples.insert("bd".into(), "Drums/Kick 1.0.wav".into());
    samples.insert("sd".into(), "Drums/Snare I.wav".into());
    samples.insert("hh".into(), "Drums/Closed Hi Hat I.wav".into());
    samples.insert("cp".into(), "Drums/Clap I.wav".into());
    samples.insert("rim".into(), "Drums/Rim I.wav".into());
    samples.insert("oh".into(), "Drums/Open Hi Hat.wav".into());
    samples.insert("sh".into(), "Drums/Shaker I.wav".into());
    samples.insert("perc".into(), "Drums/Perc I.wav".into());
    BankFile {
        bank_name: "Example".to_string(),
        base_url: "./samples/".to_string(),
        metadata: Some(PackMetadata {
            composer: Some("SongSpark".to_string()),
            album_cover_url: None,
            link: Some("https://github.com/frangedev/SongSpark".to_string()),
            description: Some("Built-in drum samples (Drums, 808, FX).".to_string()),
        }),
        samples,
    }
}

fn document_base_url() -> String {
    web_sys::window()
        .and_then(|w| w.location().href().ok())
        .and_then(|href| {
            href.rfind('/').map(|i| format!("{}/", &href[..=i]))
        })
        .unwrap_or_else(|| "./".into())
}

async fn apply_bank_samples(
    bank: &BankFile,
    base: &str,
    audio_engine: &UseStateHandle<AudioEngine>,
    samples_per_pack: &UseStateHandle<HashMap<String, Vec<String>>>,
) {
    let bank_name = bank.bank_name.clone();
    let ctx = (**audio_engine).context.clone();
    let base_trimmed = base.trim_end_matches('/');
    // Accumulate into one engine so we don't overwrite with stale state each iteration
    let mut eng = (**audio_engine).clone();
    let mut loaded_names = Vec::new();
    for (name, path) in &bank.samples {
        let path_trimmed = path.trim_start_matches('/');
        let path_encoded: String = path_trimmed
            .split('/')
            .map(|s| s.replace(' ', "%20").replace('#', "%23").replace('?', "%3F"))
            .collect::<Vec<_>>()
            .join("/");
        let full_url = format!("{}/{}", base_trimmed, path_encoded);
        if let Ok(r) = Request::get(&full_url).send().await {
            if let Ok(bytes) = r.binary().await {
                let view = js_sys::Uint8Array::from(bytes.as_slice());
                let buf = view.buffer();
                if let Ok(promise) = ctx.decode_audio_data(&buf) {
                    if let Ok(result) = JsFuture::from(promise).await {
                        let result: JsValue = result;
                        if let Ok(audio_buffer) = result.dyn_into::<AudioBuffer>() {
                            eng.load_sample(&bank_name, name, audio_buffer);
                            loaded_names.push(name.clone());
                        }
                    }
                }
            }
        }
    }
    audio_engine.set(eng);
    let mut current = (**samples_per_pack).clone();
    current.insert(bank_name, loaded_names);
    samples_per_pack.set(current);
}

/// Load the embedded example pack: no pack fetch, only sample URLs from document base + samples/.
/// Resets the example pack first so each load is a single fresh reload.
async fn load_embedded_example_pack(
    audio_engine: &UseStateHandle<AudioEngine>,
    samples_per_pack: &UseStateHandle<HashMap<String, Vec<String>>>,
    bank_loading: &UseStateHandle<bool>,
    on_pack_added: &Callback<String>,
    on_active_pack_change: &Callback<String>,
    on_metadata: &Callback<(String, Option<PackMetadata>)>,
) {
    let bank = embedded_example_bank();
    let bank_name = bank.bank_name.clone();

    // Reset: remove existing example pack so we reload once, not accumulate
    let mut eng = (**audio_engine).clone();
    eng.clear_pack(&bank_name);
    audio_engine.set(eng);
    let mut current = (**samples_per_pack).clone();
    current.remove(&bank_name);
    samples_per_pack.set(current);

    let doc_base = document_base_url();
    let base = format!("{}/samples", doc_base.trim_end_matches('/'));
    on_metadata.emit((bank_name.clone(), bank.metadata.clone()));
    on_pack_added.emit(bank_name.clone());
    let mut eng = (**audio_engine).clone();
    eng.set_active_pack(&bank_name);
    audio_engine.set(eng);
    on_active_pack_change.emit(bank_name.clone());
    apply_bank_samples(&bank, &base, audio_engine, samples_per_pack).await;
    bank_loading.set(false);
}

async fn load_bank_from_url(
    url: &str,
    audio_engine: &UseStateHandle<AudioEngine>,
    samples_per_pack: &UseStateHandle<HashMap<String, Vec<String>>>,
    bank_loading: &UseStateHandle<bool>,
    on_pack_added: &Callback<String>,
    on_active_pack_change: &Callback<String>,
    on_metadata: &Callback<(String, Option<PackMetadata>)>,
) {
    let (bank, base) = match Request::get(url).send().await {
        Ok(resp) if resp.ok() => match resp.text().await {
            Ok(text) => match serde_json::from_str::<BankFile>(&text) {
                Ok(b) => {
                    let mut base = b.base_url.trim().trim_end_matches('/').to_string();
                    if !base.starts_with("http") && !base.is_empty() {
                        let doc_base = document_base_url();
                        let path_part = base.trim_start_matches('.').trim_start_matches('/');
                        base = if path_part.is_empty() {
                            doc_base.trim_end_matches('/').into()
                        } else {
                            format!("{}/{}", doc_base.trim_end_matches('/'), path_part)
                        };
                    }
                    (b, base)
                }
                Err(_) => {
                    bank_loading.set(false);
                    return;
                }
            },
            Err(_) => {
                bank_loading.set(false);
                return;
            }
        },
        _ => {
            bank_loading.set(false);
            return;
        }
    };
    let bank_name = bank.bank_name.clone();
    on_metadata.emit((bank_name.clone(), bank.metadata.clone()));
    on_pack_added.emit(bank_name.clone());
    let mut eng = (**audio_engine).clone();
    eng.set_active_pack(&bank_name);
    audio_engine.set(eng);
    on_active_pack_change.emit(bank_name);
    apply_bank_samples(&bank, &base, audio_engine, samples_per_pack).await;
    bank_loading.set(false);
}

/// Load a pack from an already-parsed BankFile and resolved base URL (e.g. from .jsamp file).
async fn load_bank_from_parsed(
    bank: BankFile,
    base: String,
    audio_engine: &UseStateHandle<AudioEngine>,
    samples_per_pack: &UseStateHandle<HashMap<String, Vec<String>>>,
    bank_loading: &UseStateHandle<bool>,
    on_pack_added: &Callback<String>,
    on_active_pack_change: &Callback<String>,
    on_metadata: &Callback<(String, Option<PackMetadata>)>,
) {
    let bank_name = bank.bank_name.clone();
    on_metadata.emit((bank_name.clone(), bank.metadata.clone()));
    on_pack_added.emit(bank_name.clone());
    let mut eng = (**audio_engine).clone();
    eng.set_active_pack(&bank_name);
    audio_engine.set(eng);
    on_active_pack_change.emit(bank_name);
    apply_bank_samples(&bank, &base, audio_engine, samples_per_pack).await;
    bank_loading.set(false);
}

#[derive(Properties, PartialEq)]
pub struct SampleLibraryProps {
    pub audio_engine: UseStateHandle<AudioEngine>,
    pub pack_names: Vec<String>,
    pub active_pack: String,
    pub on_active_pack_change: Callback<String>,
    pub on_pack_added: Callback<String>,
    #[prop_or_default]
    pub on_sample_added: Callback<String>,
    #[prop_or(1.0)]
    pub preview_volume: f32,
    #[prop_or(0.0)]
    pub preview_pan: f32,
}

#[function_component(SampleLibrary)]
pub fn sample_library(props: &SampleLibraryProps) -> Html {
    let audio_engine = props.audio_engine.clone();
    let samples_per_pack = use_state(|| HashMap::<String, Vec<String>>::new());
    let pack_metadata = use_state(|| HashMap::<String, PackMetadata>::new());
    let new_pack_name = use_state(|| String::new());
    let bank_url = use_state(|| String::new());
    let bank_loading = use_state(|| false);
    let pack_file_input_ref = use_node_ref();

    let on_metadata = {
        let pack_metadata = pack_metadata.clone();
        Callback::from(move |(bank_name, meta): (String, Option<PackMetadata>)| {
            let mut m = (*pack_metadata).clone();
            if let Some(meta) = meta {
                m.insert(bank_name, meta);
            }
            pack_metadata.set(m);
        })
    };

    let on_active_pack_change = {
        let audio_engine = audio_engine.clone();
        let on_active_pack_change = props.on_active_pack_change.clone();
        Callback::from(move |pack: String| {
            let mut eng = (*audio_engine).clone();
            eng.set_active_pack(&pack);
            audio_engine.set(eng);
            on_active_pack_change.emit(pack);
        })
    };

    let on_pack_added = {
        let new_pack_name = new_pack_name.clone();
        let on_pack_added = props.on_pack_added.clone();
        let on_active_pack_change = on_active_pack_change.clone();
        Callback::from(move |_| {
            let name = (*new_pack_name).trim().to_string();
            if !name.is_empty() {
                on_pack_added.emit(name.clone());
                on_active_pack_change.emit(name);
                new_pack_name.set(String::new());
            }
        })
    };

    let on_new_pack_input = {
        let new_pack_name = new_pack_name.clone();
        Callback::from(move |e: InputEvent| {
            let input: HtmlInputElement = e.target_unchecked_into();
            new_pack_name.set(input.value());
        })
    };

    let on_bank_url_input = {
        let bank_url = bank_url.clone();
        Callback::from(move |e: InputEvent| {
            let input: HtmlInputElement = e.target_unchecked_into();
            bank_url.set(input.value());
        })
    };

    let on_load_example = {
        let bank_loading = bank_loading.clone();
        let audio_engine = audio_engine.clone();
        let samples_per_pack = samples_per_pack.clone();
        let on_pack_added = props.on_pack_added.clone();
        let on_active_pack_change = on_active_pack_change.clone();
        let on_metadata = on_metadata.clone();
        Callback::from(move |_| {
            bank_loading.set(true);
            let bank_loading = bank_loading.clone();
            let audio_engine = audio_engine.clone();
            let samples_per_pack = samples_per_pack.clone();
            let on_pack_added = on_pack_added.clone();
            let on_active_pack_change = on_active_pack_change.clone();
            let on_metadata = on_metadata.clone();
            wasm_bindgen_futures::spawn_local(async move {
                load_embedded_example_pack(
                    &audio_engine,
                    &samples_per_pack,
                    &bank_loading,
                    &on_pack_added,
                    &on_active_pack_change,
                    &on_metadata,
                ).await;
            });
        })
    };

    let on_load_bank = {
        let bank_url = bank_url.clone();
        let bank_loading = bank_loading.clone();
        let audio_engine = audio_engine.clone();
        let samples_per_pack = samples_per_pack.clone();
        let on_pack_added = props.on_pack_added.clone();
        let on_active_pack_change = on_active_pack_change.clone();
        let on_metadata = on_metadata.clone();
        Callback::from(move |_| {
            let url = (*bank_url).trim().to_string();
            if url.is_empty() {
                return;
            }
            bank_loading.set(true);
            let bank_loading = bank_loading.clone();
            let audio_engine = audio_engine.clone();
            let samples_per_pack = samples_per_pack.clone();
            let on_pack_added = on_pack_added.clone();
            let on_active_pack_change = on_active_pack_change.clone();
            let on_metadata = on_metadata.clone();
            wasm_bindgen_futures::spawn_local(async move {
                load_bank_from_url(
                    &url,
                    &audio_engine,
                    &samples_per_pack,
                    &bank_loading,
                    &on_pack_added,
                    &on_active_pack_change,
                    &on_metadata,
                ).await;
            });
        })
    };

    let on_load_pack_file_click = {
        let pack_file_input_ref = pack_file_input_ref.clone();
        Callback::from(move |_| {
            if let Some(input) = pack_file_input_ref.cast::<HtmlInputElement>() {
                input.click();
            }
        })
    };

    let on_pack_file_change = {
        let audio_engine = audio_engine.clone();
        let samples_per_pack = samples_per_pack.clone();
        let bank_loading = bank_loading.clone();
        let on_pack_added = props.on_pack_added.clone();
        let on_active_pack_change = on_active_pack_change.clone();
        let on_metadata = on_metadata.clone();
        Callback::from(move |e: Event| {
            let input: HtmlInputElement = e.target_unchecked_into();
            if let Some(file) = input.files().and_then(|f| f.get(0)) {
                let reader = FileReader::new().unwrap();
                let audio_engine = audio_engine.clone();
                let samples_per_pack = samples_per_pack.clone();
                let bank_loading = bank_loading.clone();
                let on_pack_added = on_pack_added.clone();
                let on_active_pack_change = on_active_pack_change.clone();
                let on_metadata = on_metadata.clone();
                let onload = wasm_bindgen::closure::Closure::wrap(Box::new(move |e: Event| {
                    let r: FileReader = e.target_unchecked_into();
                    if let Ok(Some(text)) = r.result().map(|v| v.as_string()) {
                        if let Ok(bank) = serde_json::from_str::<BankFile>(&text) {
                            let mut base = bank.base_url.trim().trim_end_matches('/').to_string();
                            if !base.starts_with("http") && !base.is_empty() {
                                let doc_base = document_base_url();
                                let path_part = base.trim_start_matches('.').trim_start_matches('/');
                                base = if path_part.is_empty() {
                                    doc_base.trim_end_matches('/').into()
                                } else {
                                    format!("{}/{}", doc_base.trim_end_matches('/'), path_part)
                                };
                            }
                            bank_loading.set(true);
                            let audio_engine = audio_engine.clone();
                            let samples_per_pack = samples_per_pack.clone();
                            let bank_loading = bank_loading.clone();
                            let on_pack_added = on_pack_added.clone();
                            let on_active_pack_change = on_active_pack_change.clone();
                            let on_metadata = on_metadata.clone();
                            wasm_bindgen_futures::spawn_local(async move {
                                load_bank_from_parsed(
                                    bank,
                                    base,
                                    &audio_engine,
                                    &samples_per_pack,
                                    &bank_loading,
                                    &on_pack_added,
                                    &on_active_pack_change,
                                    &on_metadata,
                                ).await;
                            });
                        }
                    }
                }) as Box<dyn FnMut(Event)>);
                reader.set_onload(Some(onload.as_ref().unchecked_ref()));
                onload.forget();
                reader.read_as_text(&file).unwrap();
            }
        })
    };

    let _on_file_change = {
        let audio_engine = audio_engine.clone();
        let samples_per_pack = samples_per_pack.clone();
        let active_pack = props.active_pack.clone();
        let on_active_pack_change = props.on_active_pack_change.clone();
        let on_pack_added = props.on_pack_added.clone();
        let on_sample_added = props.on_sample_added.clone();

        Callback::from(move |e: Event| {
            let input: HtmlInputElement = e.target_unchecked_into();
            let files = match input.files() {
                Some(f) => f,
                None => return,
            };
            let pack = if active_pack.is_empty() {
                "Default".to_string()
            } else {
                active_pack.clone()
            };
            if active_pack.is_empty() {
                on_pack_added.emit(pack.clone());
                let mut eng = (*audio_engine).clone();
                eng.set_active_pack(&pack);
                audio_engine.set(eng);
                on_active_pack_change.emit(pack.clone());
            }

            for i in 0..files.length() {
                if let Some(file) = files.get(i) {
                    let name = file.name();
                    let reader = FileReader::new().unwrap();
                    let audio_engine = audio_engine.clone();
                    let samples_per_pack = samples_per_pack.clone();
                    let on_sample_added = on_sample_added.clone();
                    let pack = pack.clone();

                    let onload = Closure::wrap(Box::new(move |e: Event| {
                        let reader: FileReader = e.target_unchecked_into();
                        if let Ok(ab) = reader.result() {
                            let array_buffer: js_sys::ArrayBuffer = ab.dyn_into().unwrap();
                            let ctx = (*audio_engine).context.clone();
                            let audio_engine_handle = audio_engine.clone();
                            let samples_handle = samples_per_pack.clone();
                            let on_sample_added = on_sample_added.clone();
                            let name = name.clone();
                            let pack = pack.clone();

                            wasm_bindgen_futures::spawn_local(async move {
                                let promise = ctx.decode_audio_data(&array_buffer).unwrap();
                                if let Ok(result) = JsFuture::from(promise).await {
                                    let result: JsValue = result;
                                    if let Ok(audio_buffer) = result.dyn_into::<AudioBuffer>() {
                                        let mut eng = (*audio_engine_handle).clone();
                                        eng.load_sample(&pack, &name, audio_buffer);
                                        audio_engine_handle.set(eng);

                                        let mut current = (*samples_handle).clone();
                                        current
                                            .entry(pack.clone())
                                            .or_default()
                                            .push(name.clone());
                                        samples_handle.set(current);

                                        on_sample_added.emit(name);
                                    }
                                }
                            });
                        }
                    }) as Box<dyn FnMut(Event)>);
                    reader.set_onload(Some(onload.as_ref().unchecked_ref()));
                    onload.forget();
                    reader.read_as_array_buffer(&file).unwrap();
                }
            }
        })
    };

    let current_samples = (*samples_per_pack)
        .get(&props.active_pack)
        .cloned()
        .unwrap_or_default();

    let on_play_sample = {
        let audio_engine = props.audio_engine.clone();
        let gain = props.preview_volume;
        let pan = props.preview_pan;
        Callback::from(move |name: String| {
            let eng = (*audio_engine).clone();
            eng.resume();
            let _ = eng.play_sample(&name, gain, pan);
        })
    };

    html! {
        <div class="sample-library panel">
            <h3 class="panel-title">{"Samples"}</h3>

            <div class="pack-controls">
                <label class="toolbar-label">{"Pack:"}</label>
                <select
                    class="select-preset pack-select"
                    onchange={Callback::from(move |e: Event| {
                        let select: HtmlSelectElement = e.target_unchecked_into();
                        let val = select.value();
                        if !val.is_empty() {
                            on_active_pack_change.emit(val);
                        }
                    })}
                >
                    <option value="" selected={props.active_pack.is_empty()}>{"Select pack…"}</option>
                    {for props.pack_names.iter().map(|n| {
                        let name = n.clone();
                        html! {
                            <option value={name.clone()} selected={name == props.active_pack}>
                                {name}
                            </option>
                        }
                    })}
                </select>
                <div class="pack-load-actions">
                    <button type="button" class="btn-tool" onclick={on_load_example} disabled={*bank_loading}>
                        {if *bank_loading { "…" } else { "Load sample pack" }}
                    </button>
                    <button type="button" class="btn-tool" onclick={on_load_pack_file_click} disabled={*bank_loading}>
                        {"From file"}
                    </button>
                    <input
                        ref={pack_file_input_ref}
                        type="file"
                        accept=".jsamp"
                        class="hidden-file-input"
                        onchange={on_pack_file_change}
                    />
                </div>
            </div>

            <div class="sample-library-rows">
                <div class="sample-library-row">
                    <input
                        type="text"
                        class="input-compact"
                        placeholder="New pack name"
                        value={(*new_pack_name).clone()}
                        oninput={on_new_pack_input}
                    />
                    <button type="button" class="btn-tool" onclick={on_pack_added}>{"Add pack"}</button>
                </div>
                <div class="sample-library-row">
                    <input
                        type="text"
                        class="input-compact"
                        placeholder="Pack URL (.jsamp)"
                        value={(*bank_url).clone()}
                        oninput={on_bank_url_input}
                    />
                    <button type="button" class="btn-tool" onclick={on_load_bank} disabled={*bank_loading}>
                        {"Load URL"}
                    </button>
                </div>
            </div>

            {if !props.active_pack.is_empty() {
                (*pack_metadata).get(&props.active_pack).map(|meta| {
                    html! {
                        <div class="pack-metadata">
                            {meta.album_cover_url.as_ref().map(|url| html! {
                                <img src={url.clone()} alt="Cover" class="pack-cover" />
                            })}
                            <div class="pack-meta-text">
                                {meta.composer.as_ref().map(|c| html! { <span class="pack-composer">{c}</span> })}
                                {meta.description.as_ref().map(|d| html! { <span class="pack-desc">{d}</span> })}
                                {meta.link.as_ref().map(|l| html! {
                                    <a href={l.clone()} target="_blank" rel="noopener noreferrer" class="pack-link">{"Link"}</a>
                                })}
                            </div>
                        </div>
                    }
                }).unwrap_or_default()
            } else {
                html! {}
            }}

            <p class="sample-hint sample-hint-small">{"bd, sd, hh, cp, rim, oh — "}<a href="samples/README" target="_blank" rel="noopener noreferrer">{"samples/README"}</a></p>
            <div class="samples-list">
                {current_samples.iter().map(|sample| {
                    let name = sample.clone();
                    let cb = on_play_sample.reform(move |_| name.clone());
                    html! {
                        <button type="button" class="sample-item sample-item-clickable" key={sample.clone()} onclick={cb} title="Click to play">
                            {sample}
                        </button>
                    }
                }).collect::<Html>()}
            </div>
        </div>
    }
}
