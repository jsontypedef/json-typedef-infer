//! Infers JSON Type Definition schemas from example inputs.
//!
//! JSON Type Definition, aka [RFC 8927](https://tools.ietf.org/html/rfc8927),
//! is an easy-to-learn, standardized way to define a schema for JSON data. You
//! can use JSON Typedef to portably validate data across programming languages,
//! create dummy data, generate code, and more.
//!
//! This Rust crate can generate a JSON Typedef schema from example data. If you
//! are looking to use this package as a CLI tool, see:
//!
//! https://github.com/jsontypedef/json-typedef-infer
//!
//! The remainder of these docs are focused on this crate as a Rust library, and
//! so focuses on the Rust API for using `jtd_fuzz`.
//!
//! # Quick start
//!
//! Here's how you can use this crate to infer a schema:
//!
//! ```
//! use serde_json::json;
//! use jtd_infer::{Inferrer, Hints, HintSet, NumType};
//!
//! let mut inferrer = Inferrer::new(Hints::new(
//!     NumType::Uint8,
//!     HintSet::new(vec![]),
//!     HintSet::new(vec![]),
//!     HintSet::new(vec![]),
//! ));
//!
//! inferrer = inferrer.infer(json!({ "foo": true, "bar": "xxx" }));
//! inferrer = inferrer.infer(json!({ "foo": false, "bar": null, "baz": 5 }));
//!
//! let inference = inferrer.into_schema();
//!
//! assert_eq!(
//!     json!({
//!         "properties": {
//!             "foo": { "type": "boolean" },
//!             "bar": { "type": "string", "nullable": true },
//!         },
//!         "optionalProperties": {
//!             "baz": { "type": "uint8" },
//!         },
//!     }),
//!     serde_json::to_value(inference.into_serde_schema()).unwrap(),
//! )
//! ```

mod hints;
mod inferred_number;
mod inferred_schema;

pub use crate::hints::{HintSet, Hints};
pub use crate::inferred_number::NumType;
use crate::inferred_schema::InferredSchema;
use jtd::Schema;
use serde_json::Value;

/// Keeps track of a sequence of example inputs, and can be converted into an
/// inferred schema.
pub struct Inferrer<'a> {
    inference: InferredSchema,
    hints: Hints<'a>,
}

impl<'a> Inferrer<'a> {
    /// Constructs a new inferrer with a given set of hints.
    ///
    /// See the documentation for [`Hints`] for details on what affect they have
    /// on [`Inferrer::infer`].
    pub fn new(hints: Hints<'a>) -> Self {
        Self {
            inference: InferredSchema::Unknown,
            hints,
        }
    }

    /// "Updates" the inference given an example data.
    ///
    /// Note that though the previous sentence uses the word "update", in Rust
    /// ownership terms this method *moves* `self`.
    pub fn infer(self, value: Value) -> Self {
        Self {
            inference: self.inference.infer(value, &self.hints),
            hints: self.hints,
        }
    }

    /// Converts the inference to a JSON Type Definition schema.
    ///
    /// It is guaranteed that the resulting schema will accept all of the inputs
    /// previously provided via [`Inferrer::infer`].
    pub fn into_schema(self) -> Schema {
        self.inference.into_schema(&self.hints)
    }
}
