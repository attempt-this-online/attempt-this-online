use serde::Deserialize;
use std::collections::hash_map::HashMap;

#[derive(Deserialize)]
pub struct Language {
    name: String,
    image: String,
    version: String,
    url: String,
    sbcs: bool,
    se_class: Option<String>,
}

lazy_static::lazy_static! {
    pub static ref LANGUAGES: HashMap<String, Language> = serde_json::from_str(include_str!("../languages.json")).expect("languages.json is invalid");
}
