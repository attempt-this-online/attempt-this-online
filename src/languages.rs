use serde::Deserialize;
use std::collections::hash_map::HashMap;

#[derive(Deserialize)]
pub struct Language {
    pub image: String,
    // serde silently ignores the extra fields, which we don't use
}

lazy_static::lazy_static! {
    pub static ref LANGUAGES: HashMap<String, Language> = serde_json::from_str(include_str!("../languages.json")).expect("languages.json is invalid");
}
