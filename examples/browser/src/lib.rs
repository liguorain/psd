#![feature(proc_macro_hygiene)]

use console_error_panic_hook;

use wasm_bindgen::prelude::*;
use wasm_bindgen::Clamped;
use wasm_bindgen::JsCast;
use web_sys::*;

use css_rs_macro::css;
use virtual_dom_rs::prelude::*;

use psd::Psd;

#[wasm_bindgen(start)]
pub fn main() {
    console_error_panic_hook::set_once();

    start().unwrap();
}

fn start() -> Result<(), JsValue> {
    let window = web_sys::window().unwrap();
    let document = window.document().unwrap();
    let body = document.body().unwrap();

    let psd = include_bytes!("../demo.psd");
    let psd = Psd::from_bytes(psd).unwrap();

    let mut psd_pixels = psd.rgba();

    let psd_pixels = Clamped(&mut psd_pixels[..]);
    let psd_pixels = ImageData::new_with_u8_clamped_array_and_sh(psd_pixels, psd.width(), psd.height())?;


    let app = html! {
       <div class=APP_CONTAINER>

         <div class="left-column">
           <canvas id="psd-visual" width="500" height="500">
           </canvas>
         </div>

         <div class="right-column">
           <strong>Layers</strong>
         </div>
       </div>
    };

    let mut dom_updater = DomUpdater::new_append_to_mount(app, &body);

    let canvas: HtmlCanvasElement = document
        .get_element_by_id("psd-visual")
        .unwrap()
        .dyn_into()
        .unwrap();
    let context = canvas
        .get_context("2d")?
        .unwrap()
        .dyn_into::<web_sys::CanvasRenderingContext2d>()?;

    context.put_image_data(&psd_pixels, 0., 0.)?;

    Ok(())
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
