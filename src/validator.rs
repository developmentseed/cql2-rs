use crate::Error;
use boon::{Compiler, SchemaIndex, Schemas, ValidationError};
use serde_json::Value;

/// A re-usable json-schema validator for CQL2.
#[allow(missing_debug_implementations)]
pub struct Validator {
    schemas: Schemas,
    index: SchemaIndex,
}

impl Validator {
    /// Creates a new validator.
    ///
    /// # Examples
    ///
    /// ```
    /// use cql2::Validator;
    ///
    /// let validator = Validator::new().unwrap();
    /// ```
    pub fn new() -> Result<Validator, Error> {
        let mut schemas = Schemas::new();
        let mut compiler = Compiler::new();
        let schema_json = serde_json::from_str(include_str!("cql2.json"))?;
        compiler
            .add_resource("/tmp/cql2.json", schema_json)
            .expect("the cql2 json-schema should compile");
        let index = compiler
            .compile("/tmp/cql2.json", &mut schemas)
            .expect("the cql2 json-schema should compile");
        Ok(Validator { schemas, index })
    }

    /// Validates a [serde_json::Value].
    ///
    /// # Examples
    ///
    /// ```
    /// use cql2::Validator;
    /// use serde_json::json;
    ///
    /// let validator = Validator::new().unwrap();
    ///
    /// let valid = json!({
    ///     "op": "=",
    ///     "args": [
    ///         { "property": "landsat:scene_id" },
    ///         "LC82030282019133LGN00"
    ///     ]
    /// });
    /// validator.validate(&valid).unwrap();
    ///
    /// let invalid = json!({
    ///     "op": "t_before",
    ///     "args": [{"property": "updated_at"}, {"timestamp": "invalid-timestamp"}],
    /// });
    /// validator.validate(&invalid).unwrap_err();
    /// ```
    pub fn validate<'a, 'b>(&'a self, value: &'b Value) -> Result<(), ValidationError<'a, 'b>> {
        self.schemas.validate(value, self.index)
    }
}
