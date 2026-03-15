use yew::prelude::*;
use web_sys::{HtmlCanvasElement, CanvasRenderingContext2d};
use wasm_bindgen::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;
use crate::audio::AudioEngine;

#[derive(Properties, PartialEq)]
pub struct VisualizerProps {
    #[prop_or_default]
    pub audio_engine: AudioEngine,
}

#[function_component(Visualizer)]
pub fn visualizer(props: &VisualizerProps) -> Html {
    let canvas_ref = use_node_ref();

    {
        let canvas_ref = canvas_ref.clone();
        let audio_engine = props.audio_engine.clone();

        use_effect_with_deps(
            move |_| {
                if let Some(canvas) = canvas_ref.cast::<HtmlCanvasElement>() {
                    let context = canvas
                        .get_context("2d")
                        .unwrap()
                        .unwrap()
                        .dyn_into::<CanvasRenderingContext2d>()
                        .unwrap();

                    let analyser = audio_engine.context.create_analyser().unwrap();
                    analyser.set_fft_size(2048);

                    let bin_count = analyser.frequency_bin_count() as usize;

                    let animate_holder: Rc<RefCell<Option<Closure<dyn FnMut()>>>> = Rc::new(RefCell::new(None));
                    let holder = animate_holder.clone();

                    let animate = Closure::wrap(Box::new(move || {
                        let mut data = vec![0u8; bin_count];
                        analyser.get_byte_frequency_data(&mut data);

                        let width = canvas.width() as f64;
                        let height = canvas.height() as f64;
                        let bar_width = width / bin_count as f64;

                        context.clear_rect(0.0, 0.0, width, height);
                        let _ = context.set_fill_style_str("#3498db");

                        for (i, &byte) in data.iter().enumerate() {
                            let bar_height = (byte as f64 / 255.0) * height;
                            context.fill_rect(
                                i as f64 * bar_width,
                                height - bar_height,
                                bar_width - 1.0,
                                bar_height,
                            );
                        }

                        if let Some(ref c) = *holder.borrow() {
                            let _ = web_sys::window()
                                .unwrap()
                                .request_animation_frame(c.as_ref().unchecked_ref());
                        }
                    }) as Box<dyn FnMut()>);

                    *animate_holder.borrow_mut() = Some(animate);
                    let _ = web_sys::window()
                        .unwrap()
                        .request_animation_frame(animate_holder.borrow().as_ref().unwrap().as_ref().unchecked_ref());
                }
            },
            (),
        );
    }
    
    html! {
        <div class="visualizer">
            <canvas
                ref={canvas_ref}
                width="800"
                height="200"
                class="visualizer-canvas"
            />
        </div>
    }
} 