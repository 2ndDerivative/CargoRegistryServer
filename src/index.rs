use std::{
    collections::HashMap,
    path::{PathBuf, Path}, fs::OpenOptions, io::Write,
};

use serde::{Serialize, Deserialize};
use walkdir::WalkDir;
use crate::{
    publish::PublishedPackage, 
    dependency::{DependencyKind, Dependency, ValidRegistry},
    config::{CONFIG, IndexConfigFile}
};

use self::error::WalkIndexError;

pub(crate) mod error;

#[derive(Serialize, Deserialize, Debug, Default)]
pub(crate) struct IndexCrate {
    pub(crate) name: String,
    pub(crate) vers: String,
    pub(crate) deps: Vec<IndexDependency>,
    pub(crate) cksum: String,
    pub(crate) features: HashMap<String, Vec<String>>,
    pub(crate) yanked: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) links: Option<String>,
    pub(crate) v: VValue,
    #[serde(skip_serializing_if = "HashMap::is_empty", default)]
    pub(crate) features2: HashMap<String, Vec<String>>,
}

impl IndexCrate {
    pub(crate) fn new(value: PublishedPackage, file: &[u8]) -> Self {
        type Features = HashMap<String, Vec<String>>;
        let (new_features, old_features): (Features, Features) = value.features.into_iter()
            .partition(|(_, x)| x.iter().any(|w| w.contains('?')|| w.contains(':')));
        IndexCrate {
            name: value.name, 
            vers: value.vers, 
            deps: value.deps.into_iter().map(std::convert::Into::into).collect(), 
            cksum: sha256::digest(file), 
            features: old_features, 
            yanked: false, 
            links: value.links, 
            v: if new_features.is_empty() {VValue::V1} else {VValue::V2}, 
            features2: new_features,
        }
    }
    pub fn path_in_index(&self) -> PathBuf {
        let charcount = self.name.chars().count();
        assert!(charcount > 0);
        match &self.name.to_lowercase() {
            p if charcount < 3 => PathBuf::from(charcount.to_string()).join(p),
            p if charcount == 3 => PathBuf::from("3").join(p.chars().next().expect("I just checked the length!").to_string()).join(p),
            p => {
                let mut chars = p.chars();
                PathBuf::from((&mut chars).take(2).collect::<String>())
                    .join(chars.take(2).collect::<String>())
                    .join(p)
            }
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct IndexDependency {
    name: String,
    req: String,
    features: Vec<String>,
    optional: bool,
    default_features: bool,
    target: Option<String>,
    kind: DependencyKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    registry: Option<ValidRegistry>,
    #[serde(skip_serializing_if = "Option::is_none")]
    package: Option<String>
}


impl From<Dependency> for IndexDependency {
    fn from(value: Dependency) -> Self {
        IndexDependency {
            name: value.explicit_name_in_toml.clone().unwrap_or(value.name.clone()),
            req: value.version_req,
            features: value.features,
            optional: value.optional,
            default_features: value.default_features,
            target: value.target,
            kind: value.kind,
            registry: value.registry,
            package: value.explicit_name_in_toml.is_some().then_some(value.name),
        }
    }
}

#[derive(Debug, PartialEq, Default)]
pub(crate) enum VValue {
    #[default]
    V1 = 1,
    V2 = 2,
}

impl<'de> Deserialize<'de> for VValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: serde::Deserializer<'de> {
        use serde::de::Error;
        let x = i32::deserialize(deserializer)?;
        match x {
            1 => Ok(VValue::V1),
            2 => Ok(VValue::V2),
            _ => Err(D::Error::custom("no variants specified"))
        }
    }
}

impl Serialize for VValue {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer {
        match self {
            VValue::V1 => serializer.serialize_i8(1),
            VValue::V2 => serializer.serialize_i8(2)
        }
    }
}

pub(crate) fn walk_index_crates_with_file_predicate<P: Fn(&Path) -> bool>(predicate: P) -> impl Iterator<Item = Result<IndexCrate, WalkIndexError>> {
    WalkDir::new(&CONFIG.index.path).into_iter().flatten()
    .filter(|p| 
        p.path().is_file() 
        && !p.path().starts_with(CONFIG.index.path.join(".git")) 
        && p.path() != CONFIG.index.path.join("config.json"))
    .filter(move |p| predicate(p.path()))
    .flat_map(|f| {
        let s = match std::fs::read_to_string(f.path()) {
            Err(i) => return vec![Err(WalkIndexError::IoError(i, f.into_path()))],
            Ok(s) => s,
        };
        s.lines()
            .enumerate()
            .map(|(i, line)| {
                serde_json::from_str(line)
                .map_err(|e| WalkIndexError::ParseJson(e, f.clone().into_path(), i))
            })
            .collect::<Vec<_>>()
    })
}

pub(crate) fn walk_index_crates() -> impl Iterator<Item = Result<IndexCrate, WalkIndexError>> {
    walk_index_crates_with_file_predicate(|_| true)
}

pub (crate) fn write_config_json(index_path: &Path) -> std::io::Result<()> {
    let config_path = index_path.join("config.json");
    
    let mut index_config = OpenOptions::new()
        .write(true)
        .create(true)
        .open(config_path)?;

    let json_struct: IndexConfigFile = CONFIG.clone().try_into().expect("Bad URL in configuration");
    index_config.write_all(
        serde_json::to_string_pretty(&json_struct)?
            .replace('\n', "\r\n").as_bytes()
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{IndexDependency, VValue};
    use crate::dependency::Dependency;

    #[test]
    fn dep_to_index_dep_no_rename() {
        let d = Dependency {
            name: "some_crate".to_owned(),
            explicit_name_in_toml: None,
            ..Default::default()
        };
        let transformed: IndexDependency = d.into();
        assert_eq!(transformed.package, None);
        assert_eq!(transformed.name, String::from("some_crate"));
    }

    #[test]
    fn deserialize_vvalue() {
        let j: VValue = serde_json::from_str("1").unwrap();
        assert_eq!(j, VValue::V1);
    }

    #[test]
    fn dep_to_index_dep_rename() {
        let d = Dependency {
            name: "some_crate".to_owned(),
            explicit_name_in_toml: Some("some_other_name".to_owned()),
            ..Default::default()
        };
        let transformed: IndexDependency = d.into();
        assert_eq!(transformed.package, Some("some_crate".to_string()));
        assert_eq!(transformed.name, "some_other_name".to_owned());
    }
}