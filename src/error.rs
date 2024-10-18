use thiserror::Error;

/// Crate-specific error enum.
#[derive(Debug, Error)]
#[allow(clippy::large_enum_variant)]
pub enum Error {
    /// [geojson::Error]
    #[error(transparent)]
    GeoJSON(#[from] geojson::Error),

    /// [geozero::error::GeozeroError]
    #[error(transparent)]
    Geozero(#[from] geozero::error::GeozeroError),

    /// Invalid CQL2 text
    #[error("invalid cql2-text: {0}")]
    InvalidCql2Text(String),

    /// Invalid number of arguments for the expression
    #[error("invalid number of arguments for {name}: {actual} (expected {expected})")]
    InvalidNumberOfArguments {
        /// The name of the expression or operation
        name: String,

        /// The actual number of arguments
        actual: usize,

        /// The number of arguments the expression or operation expected
        expected: usize,
    },

    /// [std::io::Error]
    #[error(transparent)]
    Io(#[from] std::io::Error),

    /// Missing argument from a function that requires one.
    #[error("function {0} is missing a required argument")]
    MissingArgument(&'static str),

    /// [std::str::ParseBoolError]
    #[error(transparent)]
    ParseBool(#[from] std::str::ParseBoolError),

    /// [std::num::ParseFloatError]
    #[error(transparent)]
    ParseFloat(#[from] std::num::ParseFloatError),

    /// [std::num::ParseIntError]
    #[error(transparent)]
    ParseInt(#[from] std::num::ParseIntError),

    /// [pest::error::Error]
    #[error(transparent)]
    Pest(#[from] Box<pest::error::Error<crate::parser::Rule>>),

    /// [serde_json::Error]
    #[error(transparent)]
    SerdeJson(#[from] serde_json::Error),

    /// A validation error.
    ///
    /// This holds a [serde_json::Value] that is the output from a
    /// [boon::ValidationError]. We can't hold the validation error itself
    /// becuase it contains references to both the validated object and the
    /// validator's data.
    #[error("validation error")]
    Validation(serde_json::Value),
}
