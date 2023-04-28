use serde::Serialize;

#[derive(Serialize, PartialEq, Debug)]
pub struct ErrorJson {
    errors: Vec<DetailedErrorJson>
}

impl<D: std::fmt::Display> From<Vec<D>> for ErrorJson {
    fn from(value: Vec<D>) -> Self {
        Self {
            errors: value
                .into_iter()
                .map(|s| DetailedErrorJson{detail: format!("{s}")})
                .collect()
        }
    }
}

impl From<ErrorJson> for Vec<u8> {
    fn from(val: ErrorJson) -> Self {
        serde_json::to_string(&val).unwrap().into_bytes()
    }
}

impl ErrorJson {
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
    use super::ErrorJson;
    #[test]
    fn error_json_two_args() {
        let words = vec!["haha", "hehe"];
        let json = ErrorJson::from(words);
        assert_eq!(serde_json::to_string(&json).unwrap(),
            r#"{"errors":[{"detail":"haha"},{"detail":"hehe"}]}"#
        );
    }
}