use yew::prelude::*;

const GITHUB_URL: &str = "https://github.com/frangedev/SongSpark";
const STRUDEL_SAMPLES: &str = "https://urswilke.github.io/strudel/learn/samples/";
const STRUDEL: &str = "https://strudel.cc";
const TIDAL: &str = "https://tidalcycles.org";

#[derive(Properties, PartialEq)]
pub struct AboutProps {
    pub on_close: Callback<()>,
}

#[function_component(About)]
pub fn about(props: &AboutProps) -> Html {
    let on_close = props.on_close.reform(|_: MouseEvent| ());

    html! {
        <div class="settings-overlay about-overlay" onclick={on_close.clone()}>
            <div class="settings-panel about-panel" onclick={|e: MouseEvent| { e.stop_propagation(); }}>
                <div class="settings-header">
                    <h2>{"About SongSpark"}</h2>
                    <button class="btn-close" type="button" onclick={on_close}>{"×"}</button>
                </div>
                <div class="about-body">
                    <p class="about-desc">
                        {"SongSpark is an open-source, browser-based live coding platform for creating music through code. Built with Rust and WebAssembly. Inspired by "}
                        <a href={STRUDEL} target="_blank" rel="noopener noreferrer">{"Strudel"}</a>
                        {" and "}
                        <a href={TIDAL} target="_blank" rel="noopener noreferrer">{"TidalCycles"}</a>
                        {" — same ideas, control patterns, sound banks and animations from code, with live control and MIDI/export."}
                    </p>
                    <p class="about-link">
                        <a href={GITHUB_URL} target="_blank" rel="noopener noreferrer">{"GitHub"}</a>
                        {" · "}
                        <a href={STRUDEL_SAMPLES} target="_blank" rel="noopener noreferrer">{"Strudel: Samples & sound banks"}</a>
                    </p>
                    <h3 class="about-section">{"Usage"}</h3>
                    <ol class="about-steps">
                        <li>{"Sound banks: add packs or "}<strong>{"Load bank from URL"}</strong>{" (JSON: bankName, baseUrl, samples). Use Strudel-style names: bd, sd, hh, cp, rim, oh, cr, rd (see "}<code>{"samples/README"}</code>{")."}</li>
                        <li>{"Switch the "}<strong>{"Pack"}</strong>{" dropdown to change which kit plays."}</li>
                        <li>{"Pattern: presets or "}<strong>{"Load"}</strong>{" a "}<code>{".jsong"}</code>{" (song: code + metadata) or "}<code>{".json"}</code>{" (session). Editor shows only code; Song info shows title/BPM/composer."}</li>
                        <li>{" "}<strong>{"Play"}</strong>{" / "}<strong>{"Spacebar"}</strong>{" = play/stop. Metronome for timing. Save = JSON; Export MIDI = pattern as MIDI."}</li>
                    </ol>
                    <h3 class="about-section">{"Roadmap"}</h3>
                    <p class="about-footer">
                        {"Closer to Strudel/Tidal: mini-notation, more pattern combinators, Web MIDI out to control hardware/software, code-driven animations (step view + BPM already in), more sampler effects."}
                    </p>
                </div>
            </div>
        </div>
    }
}
