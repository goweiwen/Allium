use std::{
    collections::HashMap,
    fs::{self, File},
};

use anyhow::{Context, Result};
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
        self.loader
            .lookup(&self.lang, key)
            .with_context(|| format!("looking up key: {}", key))
            .unwrap()
    }

    pub fn ta(&self, key: &str, args: &HashMap<String, FluentValue<'_>>) -> String {
        self.loader
            .lookup_with_args(&self.lang, key, args)
            .with_context(|| format!("looking up key: {}", key))
            .unwrap()
    }

    pub fn language(&self) -> String {
        self.lang.to_string()
    }

    pub fn languages(&self) -> Vec<String> {
        self.loader.locales().map(|i| i.to_string()).collect()
    }
}
