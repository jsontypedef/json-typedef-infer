#[derive(Debug, PartialEq)]
pub struct Hints<'a> {
    pub values_hints: Vec<&'a [String]>,
    pub discriminator_hints: Vec<&'a [String]>,
}

impl<'a> Hints<'a> {
    pub fn new(values_hints: Vec<&'a [String]>, discriminator_hints: Vec<&'a [String]>) -> Self {
        Hints {
            values_hints,
            discriminator_hints,
        }
    }

    pub fn sub_hints(&self, key: &str) -> Self {
        Hints {
            values_hints: self
                .values_hints
                .iter()
                .filter(|segments| {
                    segments.first().map(String::as_str) == Some(key)
                        || segments.first().map(String::as_str) == Some("-")
                })
                .map(|segments| &segments[1..])
                .collect(),
            discriminator_hints: self
                .discriminator_hints
                .iter()
                .filter(|segments| {
                    segments.first().map(String::as_str) == Some(key)
                        || segments.first().map(String::as_str) == Some("-")
                })
                .map(|segments| &segments[1..])
                .collect(),
        }
    }

    pub fn is_values_active(&self) -> bool {
        self.values_hints.iter().any(|segments| segments.is_empty())
    }

    pub fn is_discriminator_active(&self) -> bool {
        self.discriminator_hints
            .iter()
            .any(|segments| segments.is_empty())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sub_hints() {
        assert_eq!(
            Hints {
                values_hints: vec![
                    &["a1".to_owned(), "b1".to_owned(), "c1".to_owned()],
                    &["a1".to_owned(), "b2".to_owned(), "c1".to_owned()],
                    &["a2".to_owned(), "b1".to_owned(), "c1".to_owned()],
                    &["-".to_owned(), "b3".to_owned(), "c1".to_owned()],
                ],
                discriminator_hints: vec![
                    &["a1".to_owned(), "b1".to_owned(), "c1".to_owned()],
                    &["a1".to_owned(), "b2".to_owned(), "c1".to_owned()],
                    &["a2".to_owned(), "b1".to_owned(), "c1".to_owned()],
                    &["-".to_owned(), "b3".to_owned(), "c1".to_owned()],
                ],
            }
            .sub_hints("does-not-exist"),
            Hints {
                values_hints: vec![&["b3".to_owned(), "c1".to_owned()]],
                discriminator_hints: vec![&["b3".to_owned(), "c1".to_owned()]],
            }
        );

        assert_eq!(
            Hints {
                values_hints: vec![
                    &["a1".to_owned(), "b1".to_owned(), "c1".to_owned()],
                    &["a1".to_owned(), "b2".to_owned(), "c1".to_owned()],
                    &["a2".to_owned(), "b1".to_owned(), "c1".to_owned()],
                    &["-".to_owned(), "b3".to_owned(), "c1".to_owned()],
                ],
                discriminator_hints: vec![
                    &["a1".to_owned(), "b1".to_owned(), "c1".to_owned()],
                    &["a1".to_owned(), "b2".to_owned(), "c1".to_owned()],
                    &["a2".to_owned(), "b1".to_owned(), "c1".to_owned()],
                    &["-".to_owned(), "b3".to_owned(), "c1".to_owned()],
                ],
            }
            .sub_hints("a1"),
            Hints {
                values_hints: vec![
                    &["b1".to_owned(), "c1".to_owned()],
                    &["b2".to_owned(), "c1".to_owned()],
                    &["b3".to_owned(), "c1".to_owned()]
                ],
                discriminator_hints: vec![
                    &["b1".to_owned(), "c1".to_owned()],
                    &["b2".to_owned(), "c1".to_owned()],
                    &["b3".to_owned(), "c1".to_owned()]
                ],
            }
        );
    }

    #[test]
    fn is_values_active() {
        assert!(!Hints {
            values_hints: vec![&["a".to_owned()]],
            discriminator_hints: vec![],
        }
        .is_values_active());
        assert!(Hints {
            values_hints: vec![&["a".to_owned()]],
            discriminator_hints: vec![],
        }
        .sub_hints("a")
        .is_values_active());
        assert!(!Hints {
            values_hints: vec![&["a".to_owned()]],
            discriminator_hints: vec![],
        }
        .sub_hints("b")
        .is_values_active());
    }

    #[test]
    fn is_discriminator_active() {
        assert!(!Hints {
            values_hints: vec![],
            discriminator_hints: vec![&["a".to_owned()]],
        }
        .is_discriminator_active());
        assert!(Hints {
            values_hints: vec![],
            discriminator_hints: vec![&["a".to_owned()]],
        }
        .sub_hints("a")
        .is_discriminator_active());
        assert!(!Hints {
            values_hints: vec![],
            discriminator_hints: vec![&["a".to_owned()]],
        }
        .sub_hints("b")
        .is_discriminator_active());
    }
}
