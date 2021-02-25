use std::collections::HashMap;
use std::fmt::Display;
use std::fs::File;
use std::io::BufReader;
use std::lazy::SyncLazy;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use strfmt::strfmt;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum TranslationError {
    #[error("Language was not initialized {0}")]
    LangNotInitialized(Language),

    #[error("String '{1}' not found in {0}")]
    StringNotFound(Language, String),

    #[error("Failed to decode translation")]
    Json(#[from] serde_json::Error),

    #[error("Failed to format translation")]
    Format(#[from] strfmt::FmtError),

    #[error("IO error")]
    Io(#[from] std::io::Error),
}

static TRANSLATIONS: SyncLazy<HashMap<Language, HashMap<String, String>>> = SyncLazy::new(|| {
    let mut v = HashMap::new();
    v.insert(
        Language::English,
        load_language(Language::English).expect("Failed to load english language"),
    );
    v.insert(
        Language::Spanish,
        load_language(Language::Spanish).expect("Failed to load spanish language"),
    );
    v
});

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy)]
pub enum Language {
    English,
    Spanish,
}

impl Default for Language {
    fn default() -> Self {
        Language::English
    }
}

impl From<&Language> for &'static str {
    fn from(lang: &Language) -> &'static str {
        match lang {
            Language::English => "en",
            Language::Spanish => "es",
        }
    }
}

impl Display for Language {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let string: &str = self.into();
        write!(f, "{}", string)
    }
}

impl Language {
    pub fn get_default(key: &str) -> Result<&'static str, TranslationError> {
        Language::default().get_option(key)
    }

    pub fn get_option(self, key: &str) -> Result<&'static str, TranslationError> {
        let out = TRANSLATIONS
            .get(&self)
            .ok_or(TranslationError::LangNotInitialized(self))?
            .get(key)
            .ok_or(TranslationError::StringNotFound(self, key.into()))?;
        Ok(&out[..])
    }

    pub fn get(self, key: &str) -> Result<&'static str, TranslationError> {
        let res = self.get_option(key);
        if res.is_err() {
            return Language::get_default(key);
        }
        res
    }

    pub fn translate(self, key: &str, data: Value) -> Result<String, TranslationError> {
        let vars: HashMap<String, String> = HashMap::<String, Value>::deserialize(data)?
            .iter()
            .map(|(k, v)| {
                let v = match v {
                    Value::Number(ref v) => format!("{}", v),
                    Value::String(ref v) => v.clone(),
                    Value::Null => String::from("Null"),
                    Value::Bool(v) => format!("{}", v),
                    Value::Array(v) => format!("{:?}", v),
                    Value::Object(v) => format!("{:?}", v),
                };
                (k.clone(), v)
            })
            .collect();
        let translated = strfmt(self.get(key)?, &vars)?;
        Ok(translated)
    }
}

fn load_language(lang: Language) -> Result<HashMap<String, String>, TranslationError> {
    let file = File::open(format!("i18n/{}.json", lang))?;
    let reader = BufReader::new(file);

    let u = serde_json::from_reader(reader)?;

    Ok(u)
}
