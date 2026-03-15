use yew::prelude::*;
use crate::audio::AudioEngine;
use crate::patterns::Pattern;

#[derive(Properties, PartialEq)]
pub struct PlayerProps {
    #[prop_or_default]
    pub pattern: Option<Pattern>,
    pub audio_engine: UseStateHandle<AudioEngine>,
    pub is_playing: bool,
    pub on_toggle: Callback<()>,
}

#[function_component(Player)]
pub fn player(props: &PlayerProps) -> Html {
    let on_play = {
        let on_toggle = props.on_toggle.clone();
        Callback::from(move |_| on_toggle.emit(()))
    };

    html! {
        <div class="player">
            <button
                class={if props.is_playing { "play-button playing" } else { "play-button" }}
                onclick={on_play}
                title={if props.is_playing { "Stop" } else { "Play" }}
            >
                {if props.is_playing { "■ Stop" } else { "▶ Play" }}
            </button>
        </div>
    }
}
