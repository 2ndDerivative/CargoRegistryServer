use serde::Serialize;

#[derive(Serialize, PartialEq, Debug)]
pub struct ReturnJson {
    errors: Vec<DetailedErrorJson>
}

impl<D: std::fmt::Display> From<Vec<D>> for ReturnJson {
    fn from(value: Vec<D>) -> Self {
        Self {
            errors: value
                .into_iter()
                .map(|s| DetailedErrorJson{detail: format!("{s}")})
                .collect()
        }
    }
}

impl From<ReturnJson> for Vec<u8> {
    fn from(val: ReturnJson) -> Self {
        serde_json::to_string(&val).unwrap().into_bytes()
    }
}

impl ReturnJson {
    pub fn new<T: std::fmt::Display>(values: &[T]) -> Self {
        Self {
            errors: values
                .iter()
                .map(|s| DetailedErrorJson{detail: format!("{s}")})
                .collect()
        }
    }
}

#[derive(Serialize, PartialEq, Debug)]
struct DetailedErrorJson {
    detail: String
}

#[cfg(test)]
mod tests {
    use super::ReturnJson;
    #[test]
    fn error_json_two_args() {
        let words = vec!["haha", "hehe"];
        let json = ReturnJson::from(words);
        assert_eq!(serde_json::to_string(&json).unwrap(),
            r#"{"errors":[{"detail":"haha"},{"detail":"hehe"}]}"#
        );
    }
}