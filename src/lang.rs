use std::collections::HashMap;
use std::error::Error;
use std::fmt::Display;
use std::fs::File;
use std::io::BufReader;
use std::lazy::SyncLazy;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use strfmt::strfmt;

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
    pub fn get_default(key: &str) -> Result<&'static str, String> {
        Language::default().get_option(key)
    }

    pub fn get_option(self, key: &str) -> Result<&'static str, String> {
        TRANSLATIONS
            .get(&self)
            .ok_or(format!("{:?} language was not initialized", self))
            .map(|v| {
                v.get(key)
                    .ok_or(format!("{} was not found in {}", key, self))
            })
            .flatten()
            .map(|s| &s[..])
    }

    pub fn get(self, key: &str) -> Result<&'static str, String> {
        let res = self.get_option(key);
        if res.is_err() {
            return Language::get_default(key);
        }
        res
    }

    pub fn translate(self, key: &str, data: Value) -> Result<String, Box<dyn Error>> {
        let vars: HashMap<String, String> = HashMap::deserialize(data)?;
        let translated = strfmt(self.get(key)?, &vars)?;
        Ok(translated)
    }
}

fn load_language(lang: Language) -> Result<HashMap<String, String>, Box<dyn Error>> {
    let file = File::open(format!("i18n/{}.json", lang))?;
    let reader = BufReader::new(file);

    let u = serde_json::from_reader(reader)?;

    Ok(u)
}
