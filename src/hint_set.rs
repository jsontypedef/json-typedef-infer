const WILDCARD: &'static str = "-";

#[derive(Debug)]
pub struct HintSet<'a> {
    values: Vec<&'a [String]>,
}

impl<'a> HintSet<'a> {
    pub fn new(values: Vec<&'a [String]>) -> Self {
        HintSet { values }
    }

    pub fn sub_hints(&self, key: &str) -> Self {
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

    pub fn is_active(&self) -> bool {
        self.values.iter().any(|values| values.is_empty())
    }

    pub fn peek_active(&self) -> Option<&str> {
        self.values
            .iter()
            .find(|values| values.len() == 1)
            .and_then(|values| values.first().map(String::as_str))
    }
}
