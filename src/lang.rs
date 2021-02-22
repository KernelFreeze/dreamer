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
        Language::ENGLISH,
        load_language(Language::ENGLISH).expect("Failed to load english language"),
    );
    v.insert(
        Language::SPANISH,
        load_language(Language::SPANISH).expect("Failed to load spanish language"),
    );
    v
});

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Language {
    ENGLISH,
    SPANISH,
}

impl Default for Language {
    fn default() -> Self {
        Language::ENGLISH
    }
}

impl From<&Language> for &'static str {
    fn from(lang: &Language) -> &'static str {
        match lang {
            Language::ENGLISH => "en",
            Language::SPANISH => "es",
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
    pub fn get_default(key: &str) -> Result<&'static String, String> {
        Language::default().get_option(key)
    }

    pub fn get_option(&self, key: &str) -> Result<&'static String, String> {
        TRANSLATIONS
            .get(self)
            .ok_or(format!("{:?} language was not initialized", self))
            .map(|v| {
                v.get(key)
                    .ok_or(format!("{} was not found in {}", key, self))
            })
            .flatten()
    }

    pub fn get(&self, key: &str) -> Result<&'static String, String> {
        let res = self.get_option(key);
        if let Err(_) = res {
            return Language::get_default(key);
        }
        res
    }

    pub fn translate(&self, key: &str, data: Value) -> Result<String, Box<dyn Error>> {
        let vars: HashMap<String, String> = HashMap::deserialize(data)?;
        let translated = strfmt(&self.get(key)?, &vars)?;
        Ok(translated)
    }
}

fn load_language(lang: Language) -> Result<HashMap<String, String>, Box<dyn Error>> {
    let file = File::open(format!("i18n/{}.json", lang))?;
    let reader = BufReader::new(file);

    let u = serde_json::from_reader(reader)?;

    Ok(u)
}
