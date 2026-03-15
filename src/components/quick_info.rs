use yew::prelude::*;
use crate::patterns::Pattern;

#[derive(Properties, PartialEq)]
pub struct QuickInfoProps {
    pub bpm: u32,
    pub pattern: Pattern,
}

#[function_component(QuickInfo)]
pub fn quick_info(props: &QuickInfoProps) -> Html {
    let bpm = props.bpm as f32;
    let beats = props.pattern.duration_16ths() / 4.0;
    let sec = if bpm > 0.0 { beats * 60.0 / bpm } else { 0.0 };
    let notes = props.pattern.events.len();

    html! {
        <div class="quick-info">
            <span class="quick-info-item">{"BPM "}{props.bpm}</span>
            <span class="quick-info-sep">{" • "}</span>
            <span class="quick-info-item">{format!("{:.1} beats", beats)}</span>
            <span class="quick-info-sep">{" • "}</span>
            <span class="quick-info-item">{format!("{:.1}s", sec)}</span>
            <span class="quick-info-sep">{" • "}</span>
            <span class="quick-info-item">{notes}{" notes"}</span>
        </div>
    }
}
