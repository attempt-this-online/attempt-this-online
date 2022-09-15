use serde::Deserialize;
use std::collections::hash_map::HashMap;

#[derive(Deserialize)]
pub struct Language {
    pub name: String,
    pub image: String,
    pub version: String,
    pub url: String,
    pub sbcs: bool,
    pub se_class: Option<String>,
}

lazy_static::lazy_static! {
    pub static ref LANGUAGES: HashMap<String, Language> = serde_json::from_str(include_str!("../languages.json")).expect("languages.json is invalid");
}
