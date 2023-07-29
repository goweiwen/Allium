use std::{
    collections::HashMap,
    fmt,
    fs::{self, File},
};

use anyhow::Result;
use fluent_templates::{
    fluent_bundle::FluentValue, loader::langid, ArcLoader, LanguageIdentifier, Loader,
};
use log::{debug, warn};
use serde::{Deserialize, Serialize};

use crate::constants::{ALLIUM_LOCALES_DIR, ALLIUM_LOCALE_SETTINGS};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocaleSettings {
    pub lang: String,
}

impl Default for LocaleSettings {
    fn default() -> Self {
        Self {
            lang: "en-US".to_string(),
        }
    }
}

impl LocaleSettings {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn load() -> Result<Self> {
        if ALLIUM_LOCALE_SETTINGS.exists() {
            debug!("found state, loading from file");
            let file = File::open(ALLIUM_LOCALE_SETTINGS.as_path())?;
            if let Ok(json) = serde_json::from_reader(file) {
                return Ok(json);
            }
            warn!("failed to read locale file, removing");
            fs::remove_file(ALLIUM_LOCALE_SETTINGS.as_path())?;
        }
        Ok(Self::new())
    }

    pub fn save(&self) -> Result<()> {
        let file = File::create(ALLIUM_LOCALE_SETTINGS.as_path())?;
        serde_json::to_writer(file, &self)?;
        Ok(())
    }
}

pub struct Locale {
    pub loader: ArcLoader,
    pub lang: LanguageIdentifier,
}

impl Locale {
    pub fn new(lang: &str) -> Self {
        let loader = ArcLoader::builder(ALLIUM_LOCALES_DIR.as_path(), langid!("en-US"))
            .customize(|b| b.set_use_isolating(false))
            .build()
            .unwrap();
        let lang = lang.parse().unwrap();
        Self { loader, lang }
    }

    pub fn t(&self, key: &str) -> String {
        self.loader.lookup(&self.lang, key).unwrap_or_else(|| {
            warn!("failed to lookup key: {}", key);
            String::new()
        })
    }

    pub fn ta(&self, key: &str, args: &HashMap<String, FluentValue<'_>>) -> String {
        self.loader
            .lookup_with_args(&self.lang, key, args)
            .unwrap_or_else(|| {
                warn!("failed to lookup key: {}", key);
                String::new()
            })
    }

    pub fn language(&self) -> String {
        self.lang.to_string()
    }

    pub fn languages(&self) -> Vec<String> {
        let mut vec: Vec<_> = self.loader.locales().map(|i| i.to_string()).collect();
        vec.sort_unstable();
        vec
    }
}

impl fmt::Debug for Locale {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Locale").field("lang", &self.lang).finish()
    }
}
