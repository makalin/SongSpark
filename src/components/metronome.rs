use yew::prelude::*;
use wasm_bindgen::prelude::*;
use web_sys::{AudioContext, OscillatorNode, GainNode};
use std::cell::RefCell;

#[derive(Properties, PartialEq)]
pub struct MetronomeProps {
    #[prop_or_default]
    pub bpm: u32,
}

#[function_component(Metronome)]
pub fn metronome(props: &MetronomeProps) -> Html {
    let is_playing = use_state(|| false);
    let audio_context = use_state(|| AudioContext::new().unwrap());
    let interval_id = use_memo(|_: &()| RefCell::new(None::<i32>), ());
    
    let toggle_metronome = {
        let is_playing = is_playing.clone();
        let audio_context = audio_context.clone();
        let interval_id = interval_id.clone();
        let bpm = props.bpm;
        
        Callback::from(move |_| {
            if *is_playing {
                if let Some(id) = interval_id.borrow_mut().take() {
                    web_sys::window().unwrap().clear_interval_with_handle(id);
                }
                is_playing.set(false);
            } else {
                let context = audio_context.clone();
                let interval_ms = (60_000.0 / bpm as f32) as i32;

                let closure = Closure::wrap(Box::new(move || {
                    if let Ok(oscillator) = OscillatorNode::new(&context) {
                        if let Ok(gain) = GainNode::new(&context) {
                            oscillator.set_type(web_sys::OscillatorType::Square);
                            oscillator.frequency().set_value(880.0);
                            gain.gain().set_value(0.15);
                            let _ = oscillator.connect_with_audio_node(&gain);
                            let _ = gain.connect_with_audio_node(&context.destination());
                            let _ = oscillator.start();
                            let _ = oscillator.stop_with_when(context.current_time() + 0.03);
                        }
                    }
                }) as Box<dyn FnMut()>);

                let window = web_sys::window().unwrap();
                let id = window.set_interval_with_callback_and_timeout_and_arguments_0(
                    closure.as_ref().unchecked_ref(),
                    interval_ms,
                ).unwrap();
                closure.forget();
                *interval_id.borrow_mut() = Some(id);
                is_playing.set(true);
            }
        })
    };
    
    html! {
        <div class="metronome">
            <button
                class="btn-metronome"
                onclick={toggle_metronome}
                title={if *is_playing { "Stop metronome" } else { "Start metronome" }}
            >
                {if *is_playing { "■ Stop" } else { "⏱ Tick" }}
            </button>
        </div>
    }
}
