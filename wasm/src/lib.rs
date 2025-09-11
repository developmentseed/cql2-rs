use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name = CQL2)]
pub struct CQL2Expression(cql2::Expr);

#[wasm_bindgen(js_class = CQL2)]
impl CQL2Expression {
    #[wasm_bindgen(constructor)]
    pub fn new(v: String) -> Result<CQL2Expression, JsError> {
        let e: cql2::Expr = v.parse()?;
        Ok(CQL2Expression(e))
    }

    pub fn is_valid(&self) -> bool {
        self.0.is_valid()
    }

    pub fn to_json(&self) -> Result<String, JsError> {
        let r = self.0.to_json()?;
        Ok(r)
    }

    pub fn to_json_pretty(&self) -> Result<String, JsError> {
        let r = self.0.to_json_pretty()?;
        Ok(r)
    }

    pub fn to_text(&self) -> Result<String, JsError> {
        let r = self.0.to_text()?;
        Ok(r)
    }

    pub fn reduce(&self) -> Result<CQL2Expression, JsError> {
        let r = self.0.clone().reduce(None)?;
        Ok(CQL2Expression(r))
    }
}
