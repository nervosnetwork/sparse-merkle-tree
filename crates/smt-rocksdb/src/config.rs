use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoreConfig {
    #[serde(default)]
    pub path: PathBuf,
    #[serde(default)]
    pub cache_size: Option<usize>,
    #[serde(default)]
    pub options_file: Option<PathBuf>,
    #[serde(default)]
    pub options: HashMap<String, String>,
}
