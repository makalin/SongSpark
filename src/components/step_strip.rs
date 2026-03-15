use yew::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;

const STEP_COUNT: usize = 16;

#[derive(Properties, PartialEq)]
pub struct StepStripProps {
    pub is_playing: bool,
    pub bpm: u32,
}

#[function_component(StepStrip)]
pub fn step_strip(props: &StepStripProps) -> Html {
    let step = use_state(|| 0usize);
    let step_handle = step.clone();
    // Ref so the interval callback always reads the latest step (state updates are async)
    let step_ref: Rc<RefCell<usize>> = Rc::new(RefCell::new(0));

    use_effect_with_deps(
        move |(is_playing, bpm)| {
            let id_holder: Rc<RefCell<Option<i32>>> = Rc::new(RefCell::new(None));
            let id_holder_cleanup = id_holder.clone();
            let step_ref = step_ref.clone();
            if *is_playing {
                let step_handle = step_handle.clone();
                *step_ref.borrow_mut() = 0;
                step_handle.set(0);
                let interval_ms = 60_000 / (*bpm as u32).max(40) / 4;
                if let Some(w) = web_sys::window() {
                    let closure = Closure::wrap(Box::new(move || {
                        let mut cur = step_ref.borrow_mut();
                        *cur = (*cur + 1) % STEP_COUNT;
                        let next = *cur;
                        step_handle.set(next);
                    }) as Box<dyn FnMut()>);
                    if let Ok(id) = w.set_interval_with_callback_and_timeout_and_arguments_0(
                        closure.as_ref().unchecked_ref(),
                        interval_ms as i32,
                    ) {
                        closure.forget();
                        *id_holder.borrow_mut() = Some(id);
                    }
                }
            } else {
                *step_ref.borrow_mut() = 0;
                step_handle.set(0);
            }
            move || {
                if let Some(id) = id_holder_cleanup.borrow_mut().take() {
                    if let Some(w) = web_sys::window() {
                        let _ = w.clear_interval_with_handle(id);
                    }
                }
            }
        },
        (props.is_playing, props.bpm),
    );

    let current = *step;
    html! {
        <div class="step-strip" aria-label="Step indicator">
            {for (0..STEP_COUNT).map(|i| {
                let active = i == current && props.is_playing;
                html! {
                    <div
                        class={if active { "step-cell step-cell-active" } else { "step-cell" }}
                        key={i}
                    />
                }
            })}
        </div>
    }
}
