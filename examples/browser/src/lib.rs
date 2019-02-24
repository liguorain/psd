#![feature(proc_macro_hygiene)]

use console_error_panic_hook;

use wasm_bindgen::prelude::*;
use wasm_bindgen::Clamped;
use wasm_bindgen::JsCast;
use web_sys::*;

use css_rs_macro::css;
use virtual_dom_rs::prelude::*;

use psd::Psd;
use std::cell::RefCell;
use std::collections::HashMap;
use std::ops::Deref;
use std::rc::Rc;

#[wasm_bindgen]
struct App {
    store: Rc<RefCell<Store>>,
    dom_updater: DomUpdater,
    raf_closure_holder: Rc<RefCell<Option<Box<dyn AsRef<JsValue>>>>>,
}

struct Store {
    state: State,
    on_msg: Option<Box<Fn()>>,
}

impl Deref for Store {
    type Target = State;

    fn deref(&self) -> &Self::Target {
        &self.state
    }
}

struct State {
    psd: Psd,
    // Layer name, whether or not it is visible
    layer_visibility: HashMap<String, bool>,
}

impl Store {
    fn msg(&mut self, msg: &Msg) {
        self.state.msg(msg);
        self.on_msg.as_ref().unwrap()();
    }
}

impl State {
    fn msg(&mut self, msg: &Msg) {
        match msg {
            Msg::ReplacePsd(psd) => {}
            Msg::SetLayerVisibility(idx, visible) => {
                let visibility = self
                    .layer_visibility
                    .get_mut(self.psd.layer_by_idx(*idx).unwrap().name())
                    .unwrap();

                *visibility = *visible;
            }
        }
    }
}

enum Msg {
    ReplacePsd(Psd),
    /// Set whether or not a layer (by index) should be visible
    SetLayerVisibility(usize, bool),
}

#[wasm_bindgen]
struct AppWrapper(Rc<RefCell<App>>);

#[wasm_bindgen]
impl AppWrapper {
    #[wasm_bindgen(constructor)]
    pub fn new() -> AppWrapper {
        let app = App::start().unwrap();

        let closure_holder = Rc::clone(&app.raf_closure_holder);

        let store = Rc::clone(&app.store);

        let app = Rc::new(RefCell::new(app));
        let app_clone = Rc::clone(&app);

        let on_msg = move || {
            let store = Rc::clone(&store);
            let app = Rc::clone(&app);
            let closure_holder = Rc::clone(&closure_holder);

            let render = move || {
                let store = Rc::clone(&store);
                let app = Rc::clone(&app);

                let vdom = Renderer::render(store);
                app.borrow_mut().update(vdom);
            };
            let mut callback = Closure::wrap(Box::new(render) as Box<FnMut()>);
            web_sys::window()
                .unwrap()
                .request_animation_frame(&callback.as_ref().unchecked_ref());

            *closure_holder.borrow_mut() = Some(Box::new(callback));
        };

        app_clone.borrow_mut().store.borrow_mut().on_msg = Some(Box::new(on_msg));

        AppWrapper(app_clone)
    }
}

#[wasm_bindgen]
impl App {
    pub fn start() -> Result<App, JsValue> {
        console_error_panic_hook::set_once();

        let psd = include_bytes!("../demo.psd");
        let psd = Psd::from_bytes(psd).unwrap();

        let mut layer_visibility = HashMap::new();
        for layer in psd.layers().iter() {
            layer_visibility.insert(layer.name().to_string(), true);
        }

        let window = web_sys::window().unwrap();
        let document = window.document().unwrap();
        let body = document.body().unwrap();

        let app = html! { <div> </div> };
        let mut dom_updater = DomUpdater::new_append_to_mount(app, &body);

        let state = State {
            psd,
            layer_visibility,
        };

        let on_msg = None;
        let store = Store { state, on_msg };
        let store = Rc::new(RefCell::new(store));

        let vdom = Renderer::render(Rc::clone(&store));

        let mut app = App {
            store,
            dom_updater,
            raf_closure_holder: Rc::new(RefCell::new(None)),
        };

        app.update(vdom);

        Ok(app)
    }

    fn update(&mut self, vdom: VirtualNode) -> Result<(), JsValue> {
        self.dom_updater.update(vdom);

        let psd = &self.store.borrow().psd;

        let mut psd_pixels = psd
            .flatten_layers_rgba(&|(idx, layer)| {
                let layer_visible = *self
                    .store
                    .borrow()
                    .layer_visibility
                    .get(layer.name())
                    .unwrap();

                layer_visible
            })
            .unwrap();

        let psd_pixels = Clamped(&mut psd_pixels[..]);
        let psd_pixels =
            ImageData::new_with_u8_clamped_array_and_sh(psd_pixels, psd.width(), psd.height())?;

        let window = web_sys::window().unwrap();
        let document = window.document().unwrap();
        let body = document.body().unwrap();

        let canvas: HtmlCanvasElement = document
            .get_element_by_id("psd-visual")
            .unwrap()
            .dyn_into()?;
        let context = canvas
            .get_context("2d")?
            .unwrap()
            .dyn_into::<web_sys::CanvasRenderingContext2d>()?;

        context.put_image_data(&psd_pixels, 0., 0.)?;

        Ok(())
    }
}

struct Renderer {}

impl Renderer {
    fn render(store: Rc<RefCell<Store>>) -> VirtualNode {
        let store_clone = Rc::clone(&store);

        let store = store.borrow();

        let mut layers: Vec<VirtualNode> = store
            .psd
            .layers()
            .iter()
            .enumerate()
            .map(|(idx, layer)| {
                let store = Rc::clone(&store_clone);

                let checked = *store.borrow().layer_visibility.get(layer.name()).unwrap();

                let checked = if checked { "true" } else { "false" };

                html! {
                <div
                    style="cursor: pointer; padding-top: 5px; padding-bottom: 5px;"
                >
                  <label style="cursor: pointer;">
                    <span style="cursor: pointer;">{ text!(layer.name()) }</span>
                    <input
                     type="checkbox"
                     checked=checked
                     // TODO: make virtual-dom-rs allow for variables .. `onchange=onchange`
                     // To be able to move the callback outside of the html macro..
                     //
                     // If the attribute starts with `on` treat the value as a closure.
                     onchange=move |event: web_sys::Event| {
                       let input: HtmlInputElement = event.target().unwrap().dyn_into().unwrap();

                       let msg = Msg::SetLayerVisibility(idx, input.checked());
                       store.borrow_mut().msg(&msg);
                     }
                     >
                  </label>
                </div>
                }
            })
            .collect();
        layers.reverse();

        let vdom = html! {
           <div class=APP_CONTAINER>

             <div class="left-column">
               <canvas id="psd-visual" width="500" height="500"></canvas>
             </div>

             <div class="right-column">
               <strong>Layers</strong>
               { layers }
             </div>
           </div>
        };

        vdom
    }
}

static APP_CONTAINER: &'static str = css! {r#"
:host {
    display: flex;
}
"#};

static _LAYOUT: &'static str = css! {r#"
.left-column {
}

.right-column {
}
"#};

// A macro to provide `println!(..)`-style syntax for `console.log` logging.
// TODO: cfg debug attrs so that logs don't make it into production builds
#[macro_export]
macro_rules! clog {
    ( $( $t:tt )* ) => {
        web_sys::console::log_1(&format!( $( $t )* ).into());
    }
}