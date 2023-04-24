use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter, Result as FMTResult};

use crate::config::CONFIG;


#[derive(Deserialize, Debug)]
pub(crate) struct Dependency {
    pub(crate) name: String,
    pub(crate) version_req: String,
    pub(crate) features: Vec<String>,
    pub(crate) optional: bool,
    pub(crate) default_features: bool,
    pub(crate) target: Option<String>,
    pub(crate) kind: DependencyKind,
    pub(crate) registry: Option<ValidRegistry>,
    pub(crate) explicit_name_in_toml: Option<String>,
}

impl Default for Dependency {
    fn default() -> Self {
        Dependency { 
            name: String::new(), 
            version_req: String::new(), 
            features: vec![], 
            optional: false, 
            default_features: true, 
            target: None, 
            kind: DependencyKind::Normal, 
            registry: None, 
            explicit_name_in_toml: None 
        }
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub(crate) enum DependencyKind {
    Dev,
    Build,
    Normal,
}

#[derive(Deserialize, Debug, Serialize)]
#[non_exhaustive]
pub(crate) enum ValidRegistry {
    This,
    #[serde(rename = "https://github.com/rust-lang/crates.io-index")]
    CratesIO,
}

impl Display for ValidRegistry {
    fn fmt(&self, f: &mut Formatter<'_>) -> FMTResult {
        match self {
            Self::CratesIO => write!(f, "https://github.com/rust-lang/crates.io-index"),
            Self::This => write!(f, "{}", &CONFIG.index.path.display())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::DependencyKind;

    #[test]
    fn deserialize_dependencykind_normal() {
        let d: DependencyKind = serde_json::from_str("\"normal\"").unwrap();
        assert_eq!(d, DependencyKind::Normal);
    }

    #[test]
    fn deserialize_dependencykind_dev() {
        let d: DependencyKind = serde_json::from_str("\"dev\"").unwrap();
        assert_eq!(d, DependencyKind::Dev);
    }
    #[test]
    fn deserialize_dependencykind_build() {
        let d: DependencyKind = serde_json::from_str("\"build\"").unwrap();
        assert_eq!(d, DependencyKind::Build);
    }
    #[test]
    fn deserialize_dependencykind_negative() {
        let r: Result<DependencyKind, _> = serde_json::from_str("\"anything\"");
        assert!(r.is_err());
    }
}
