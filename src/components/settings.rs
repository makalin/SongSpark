use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct SettingsProps {
    pub bpm: u32,
    pub on_bpm_change: Callback<u32>,
    pub theme: String,
    pub on_theme_change: Callback<String>,
    pub on_close: Callback<()>,
}

/// Normalize legacy "dark"/"light" to theme id used in CSS. Returns owned String for comparison.
fn theme_id(theme: &str) -> String {
    match theme {
        "dark" => "spark-dark".into(),
        "light" => "spark-light".into(),
        "spark-dark" | "spark-light" | "forest-dark" | "forest-light"
        | "ocean-dark" | "ocean-light" | "sunset-dark" | "sunset-light"
        | "mono-dark" | "mono-light" | "rose-dark" | "rose-light" => theme.to_string(),
        _ => "spark-dark".into(),
    }
}

const THEME_OPTIONS: &[(&str, &str, &str)] = &[
    ("spark-dark", "Spark", "Dark"),
    ("spark-light", "Spark", "Light"),
    ("forest-dark", "Forest", "Dark"),
    ("forest-light", "Forest", "Light"),
    ("ocean-dark", "Ocean", "Dark"),
    ("ocean-light", "Ocean", "Light"),
    ("sunset-dark", "Sunset", "Dark"),
    ("sunset-light", "Sunset", "Light"),
    ("mono-dark", "Mono", "Dark"),
    ("mono-light", "Mono", "Light"),
    ("rose-dark", "Rose", "Dark"),
    ("rose-light", "Rose", "Light"),
];

#[function_component(Settings)]
pub fn settings(props: &SettingsProps) -> Html {
    let bpm = use_state(|| props.bpm);
    let bpm_handle = bpm.clone();

    let on_bpm_input = {
        let bpm = bpm.clone();
        let on_bpm_change = props.on_bpm_change.clone();
        Callback::from(move |e: InputEvent| {
            let input: web_sys::HtmlInputElement = e.target_unchecked_into();
            if let Ok(v) = input.value().parse::<u32>() {
                if (40..=240).contains(&v) {
                    bpm.set(v);
                    on_bpm_change.emit(v);
                }
            }
        })
    };

    let current = theme_id(&props.theme);
    let on_close_click = props.on_close.reform(|_: MouseEvent| ());

    html! {
        <div class="settings-overlay" onclick={on_close_click.clone()}>
            <div class="settings-panel" onclick={|e: MouseEvent| { e.stop_propagation(); }}>
                <div class="settings-header">
                    <h2>{"SETTINGS"}</h2>
                    <button class="btn-close" onclick={on_close_click}>{"×"}</button>
                </div>
                <div class="settings-body">
                    <div class="setting-row">
                        <label>{"BPM"}</label>
                        <input
                            type="number"
                            min="40"
                            max="240"
                            value={bpm_handle.to_string()}
                            oninput={on_bpm_input}
                            class="input-num"
                        />
                    </div>
                    <div class="setting-row setting-row-theme">
                        <label>{"Theme"}</label>
                        <div class="theme-grid">
                            {for THEME_OPTIONS.chunks(2).map(|chunk| {
                                let (id1, family1, mode1) = chunk[0];
                                let (id2, _family2, mode2) = if chunk.len() > 1 { chunk[1] } else { chunk[0] };
                                let family_label = family1.to_string();
                                let id1_s = (*id1).to_string();
                                let id2_s = (*id2).to_string();
                                let emit1 = id1_s.clone();
                                let emit2 = id2_s.clone();
                                let on_theme = props.on_theme_change.clone();
                                let is_active1 = current == id1_s;
                                let is_active2 = current == id2_s;
                                html! {
                                    <div class="theme-row" key={family_label.clone()}>
                                        <span class="theme-family-label">{family_label}</span>
                                        <div class="theme-buttons">
                                            <button
                                                type="button"
                                                class={if is_active1 { "btn-theme active" } else { "btn-theme" }}
                                                onclick={on_theme.reform(move |_| emit1.clone())}
                                            >
                                                {"☽ "}{mode1}
                                            </button>
                                            <button
                                                type="button"
                                                class={if is_active2 { "btn-theme active" } else { "btn-theme" }}
                                                onclick={on_theme.reform(move |_| emit2.clone())}
                                            >
                                                {"☀ "}{mode2}
                                            </button>
                                        </div>
                                    </div>
                                }
                            })}
                        </div>
                    </div>
                </div>
            </div>
        </div>
    }
}
