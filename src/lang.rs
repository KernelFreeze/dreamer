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
    pub fn get_default<S>(key: S) -> Result<&'static str, TranslationError>
    where
        S: AsRef<str>,
    {
        Language::default().get_option::<S>(key)
    }

    pub fn get_option<S>(self, key: S) -> Result<&'static str, TranslationError>
    where
        S: AsRef<str>,
    {
        let out = TRANSLATIONS
            .get(&self)
            .ok_or(TranslationError::LangNotInitialized(self))?
            .get(key.as_ref())
            .ok_or_else(|| TranslationError::StringNotFound(self, String::from(key.as_ref())))?;
        Ok(&out[..])
    }

    pub fn get<'a, S>(self, key: S) -> &'a str
    where
        S: Into<&'a str>,
    {
        let key = key.into();
        self.get_option(&key)
            .unwrap_or_else(|_| Language::get_default(&key).unwrap_or(key))
    }

    pub fn translate<S>(self, key: S, data: Value) -> Result<String, TranslationError>
    where
        S: AsRef<str>,
    {
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
        let translated = strfmt(&self.get(key.as_ref()), &vars)?;
        Ok(translated)
    }
}

fn load_language(lang: Language) -> Result<HashMap<String, String>, TranslationError> {
    let file = File::open(format!("i18n/{}.json", lang))?;
    let reader = BufReader::new(file);

    let u = serde_json::from_reader(reader)?;

    Ok(u)
}
