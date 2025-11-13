use cql2::ToSqlAst;
use wasm_bindgen::prelude::*;

/// Parse CQL2 text format
#[wasm_bindgen(js_name = parseText)]
pub fn parse_text(s: &str) -> Result<CQL2Expression, JsError> {
    let expr = cql2::parse_text(s)?;
    Ok(CQL2Expression(expr))
}

/// Parse CQL2 JSON format
#[wasm_bindgen(js_name = parseJson)]
pub fn parse_json(s: &str) -> Result<CQL2Expression, JsError> {
    let expr = cql2::parse_json(s)?;
    Ok(CQL2Expression(expr))
}

#[wasm_bindgen(js_name = Expr)]
pub struct CQL2Expression(cql2::Expr);

#[wasm_bindgen(js_class = Expr)]
impl CQL2Expression {
    #[wasm_bindgen(constructor)]
    pub fn new(v: String) -> Result<CQL2Expression, JsError> {
        let e: cql2::Expr = v.parse()?;
        Ok(CQL2Expression(e))
    }

    /// Validate the CQL2 expression against the JSON schema
    pub fn validate(&self) -> Result<(), JsError> {
        let validator = cql2::Validator::new()?;
        let value = self.0.to_value()?;
        validator.validate(&value)?;
        Ok(())
    }

    /// Check if the expression is valid (deprecated, use validate() instead)
    pub fn is_valid(&self) -> bool {
        self.0.is_valid()
    }

    /// Check if the expression matches the given item
    ///
    /// # Arguments
    /// * `item` - JavaScript object representing the item to match against
    pub fn matches(&self, item: JsValue) -> Result<bool, JsError> {
        let value = if item.is_null() || item.is_undefined() {
            None
        } else {
            Some(serde_wasm_bindgen::from_value(item)?)
        };
        Ok(self.0.clone().matches(value.as_ref())?)
    }

    /// Reduce the expression, optionally with an item context
    ///
    /// # Arguments
    /// * `item` - JavaScript object representing the item context for reduction
    pub fn reduce(&self, item: JsValue) -> Result<CQL2Expression, JsError> {
        let value = if item.is_null() || item.is_undefined() {
            None
        } else {
            Some(serde_wasm_bindgen::from_value(item)?)
        };
        let r = self.0.clone().reduce(value.as_ref())?;
        Ok(CQL2Expression(r))
    }

    pub fn to_json(&self) -> Result<JsValue, JsError> {
        let r = self.0.to_json()?;
        let js_value = serde_wasm_bindgen::to_value(&r)?;
        Ok(js_value)
    }
    
    pub fn to_json_pretty(&self) -> Result<String, JsError> {
        let r = self.0.to_json_pretty()?;
        Ok(r)
    }
    
    pub fn to_text(&self) -> Result<String, JsError> {
        let r = self.0.to_text()?;
        Ok(r)
    }

    /// Convert the expression to SQL
    pub fn to_sql(&self) -> Result<String, JsError> {
        Ok(self.0.to_sql()?)
    }

    /// Add two expressions together (AND operation)
    pub fn add(&self, other: &CQL2Expression) -> CQL2Expression {
        CQL2Expression(self.0.clone() + other.0.clone())
    }

    /// Check if two expressions are equal
    pub fn equals(&self, other: &CQL2Expression) -> bool {
        self.0 == other.0
    }
}
