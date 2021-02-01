use crate::inferred_number::NumType;

/// Hints for [`Inferrer`][`crate::Inferrer`].
///
/// By default, [`Inferrer`][`crate::Inferrer`] will never produce enum, values,
/// or discriminator forms. Hints tell [`Inferrer`][`crate::Inferrer`] to use
/// these forms. See [`HintSet`] for details on how you can specify the "paths"
/// to the pieces of the input that should use these forms.
///
/// `default_num_type` tells [`Inferrer`][`crate::Inferrer`] what numeric type
/// to attempt to use by default when it encounters a JSON number. This default
/// will be ignored if it doesn't contain the example data. When the default is
/// ignored, the inferrer will infer the narrowest numerical type possible for
/// input data, preferring unsigned integers over signed integers.
///
/// To adapt the example used at [the crate-level docs][`crate`], here's how you
/// could change [`Inferrer`][`crate::Inferrer`] behavior using hints:
///
/// ```
/// use serde_json::json;
/// use jtd_infer::{Inferrer, Hints, HintSet, NumType};
///
/// let enum_path = vec!["bar".to_string()];
/// let mut inferrer = Inferrer::new(Hints::new(
///     NumType::Float32,
///     HintSet::new(vec![&enum_path]),
///     HintSet::new(vec![]),
///     HintSet::new(vec![]),
/// ));
///
/// inferrer = inferrer.infer(json!({ "foo": true, "bar": "xxx" }));
/// inferrer = inferrer.infer(json!({ "foo": false, "bar": null, "baz": 5 }));
///
/// let inference = inferrer.into_schema();
///
/// assert_eq!(
///     json!({
///         "properties": {
///             "foo": { "type": "boolean" },
///             "bar": { "enum": ["xxx"], "nullable": true }, // now an enum
///         },
///         "optionalProperties": {
///             "baz": { "type": "float32" }, // instead of uint8
///         },
///     }),
///     serde_json::to_value(inference.into_serde_schema()).unwrap(),
/// )
/// ```
pub struct Hints<'a> {
    default_num_type: NumType,
    enums: HintSet<'a>,
    values: HintSet<'a>,
    discriminator: HintSet<'a>,
}

impl<'a> Hints<'a> {
    /// Constructs a new set of [`Hints`].
    pub fn new(
        default_num_type: NumType,
        enums: HintSet<'a>,
        values: HintSet<'a>,
        discriminator: HintSet<'a>,
    ) -> Self {
        Hints {
            default_num_type,
            enums,
            values,
            discriminator,
        }
    }

    pub(crate) fn default_num_type(&self) -> &NumType {
        &self.default_num_type
    }

    pub(crate) fn sub_hints(&self, key: &str) -> Self {
        Self::new(
            self.default_num_type.clone(),
            self.enums.sub_hints(key),
            self.values.sub_hints(key),
            self.discriminator.sub_hints(key),
        )
    }

    pub(crate) fn is_enum_active(&self) -> bool {
        self.enums.is_active()
    }

    pub(crate) fn is_values_active(&self) -> bool {
        self.values.is_active()
    }

    pub(crate) fn peek_active_discriminator(&self) -> Option<&str> {
        self.discriminator.peek_active()
    }
}

const WILDCARD: &'static str = "-";

/// A set of paths to parts of the input that are subject to a hint in
/// [`Hints`].
pub struct HintSet<'a> {
    values: Vec<&'a [String]>,
}

impl<'a> HintSet<'a> {
    /// Constructs a new [`HintSet`].
    ///
    /// Each element of `values` is a separate "path". Each element of a path is
    /// treated as a path "segment". So, for example, this:
    ///
    /// ```
    /// use jtd_infer::HintSet;
    ///
    /// let path1 = vec!["foo".to_string(), "bar".to_string()];
    /// let path2 = vec!["baz".to_string()];
    /// HintSet::new(vec![&path1, &path2]);
    /// ```
    ///
    /// Creates a set of paths pointing to `/foo/bar` and `/baz` in an input.
    ///
    /// The `-` path segment value is special, and acts as a wildcard, matching
    /// any property name. It also matches array elements, unlike ordinary path
    /// segments.
    pub fn new(values: Vec<&'a [String]>) -> Self {
        HintSet { values }
    }

    pub(crate) fn sub_hints(&self, key: &str) -> Self {
        Self::new(
            self.values
                .iter()
                .filter(|values| {
                    let first = values.first().map(String::as_str);
                    first == Some(WILDCARD) || first == Some(key)
                })
                .map(|values| &values[1..])
                .collect(),
        )
    }

    pub(crate) fn is_active(&self) -> bool {
        self.values.iter().any(|values| values.is_empty())
    }

    pub(crate) fn peek_active(&self) -> Option<&str> {
        self.values
            .iter()
            .find(|values| values.len() == 1)
            .and_then(|values| values.first().map(String::as_str))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hint_set() {
        let path = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        let hint_set = HintSet::new(vec![&path]);
        assert!(!hint_set.is_active());
        assert_eq!(None, hint_set.peek_active());

        assert!(!hint_set.sub_hints("a").is_active());
        assert_eq!(None, hint_set.sub_hints("a").peek_active());

        assert!(!hint_set.sub_hints("a").sub_hints("b").is_active());
        assert_eq!(
            Some("c"),
            hint_set.sub_hints("a").sub_hints("b").peek_active()
        );

        assert!(hint_set
            .sub_hints("a")
            .sub_hints("b")
            .sub_hints("c")
            .is_active());

        assert_eq!(
            None,
            hint_set
                .sub_hints("a")
                .sub_hints("b")
                .sub_hints("c")
                .peek_active()
        );
    }

    #[test]
    fn hint_set_wildcard() {
        let path1 = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        let path2 = vec!["d".to_string(), "-".to_string(), "e".to_string()];
        let hint_set = HintSet::new(vec![&path1, &path2]);

        assert!(!hint_set
            .sub_hints("a")
            .sub_hints("x")
            .sub_hints("c")
            .is_active());

        assert!(hint_set
            .sub_hints("d")
            .sub_hints("x")
            .sub_hints("e")
            .is_active());
    }
}
