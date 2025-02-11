use wasm_bindgen::prelude::*;
use cql2;

#[wasm_bindgen(start)]
fn main() -> Result<(), JsValue> {
    // Use `web_sys`'s global `window` function to get a handle on the global
    // window object.
    let window = web_sys::window().expect("no global `window` exists");
    let document = window.document().expect("should have a document on window");
    let body = document.body().expect("document should have a body");

    // Manufacture the element we're gonna append
    let val = document.create_element("p")?;
    val.set_inner_html("Hello from Rust!");

    body.append_child(&val)?;

    Ok(())
}

#[wasm_bindgen(js_name = CQL2)]
pub struct CQL2Expression(cql2::Expr);

#[wasm_bindgen(js_class = CQL2)]
impl CQL2Expression {
    #[wasm_bindgen(constructor)]
    pub fn new(v: String) -> CQL2Expression {
        let e: cql2::Expr = v.parse().expect("Could not parse CQL2");
        CQL2Expression(e)
    }

    pub fn is_valid(&self) -> bool {
        self.0.is_valid()
    }

    pub fn to_json(&self) -> String {
        self.0.to_json().expect("Could not convert to json.")
    }

    pub fn to_json_pretty(&self) -> String {
        self.0.to_json_pretty().expect("Could not convert to json.")
    }

    pub fn to_text(&self) -> String {
        self.0.to_text().expect("Could not convert to text.")
    }
}
