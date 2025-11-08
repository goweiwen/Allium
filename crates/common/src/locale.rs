use std::{
    borrow::Cow,
    collections::HashMap,
    fmt,
    fs::{self, File},
};

use anyhow::Result;
use fluent_templates::{
    ArcLoader, LanguageIdentifier, Loader, fluent_bundle::FluentValue, loader::langid,
};
use log::{debug, warn};
use serde::{Deserialize, Serialize};

use crate::constants::{ALLIUM_LOCALE_SETTINGS, ALLIUM_LOCALES_DIR, ALLIUM_THEMES_DIR};
use crate::stylesheet::Theme;

pub use fluent_templates::fluent_bundle::FluentValue as LocaleFluentValue;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocaleSettings {
    pub lang: String,
}

impl Default for LocaleSettings {
    fn default() -> Self {
        Self {
            lang: "en-US".into(),
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
    pub default_loader: ArcLoader,
    pub theme_loader: Option<ArcLoader>,
    pub lang: LanguageIdentifier,
}

impl Locale {
    pub fn new(lang: &str) -> Self {
        let theme = Theme::load();
        let theme_locales_dir = ALLIUM_THEMES_DIR.join(&theme.0).join("locales");

        let default_loader = ArcLoader::builder(ALLIUM_LOCALES_DIR.as_path(), langid!("en-US"))
            .customize(|b| b.set_use_isolating(false))
            .build()
            .unwrap();

        let theme_loader = if theme_locales_dir.exists()
            && theme_locales_dir
                .read_dir()
                .map(|mut d| d.next().is_some())
                .unwrap_or(false)
        {
            debug!(
                "loading locale overrides from theme: {}",
                theme_locales_dir.display()
            );
            Some(
                ArcLoader::builder(&theme_locales_dir, langid!("en-US"))
                    .customize(|b| b.set_use_isolating(false))
                    .build()
                    .unwrap(),
            )
        } else {
            None
        };

        let lang = lang.parse().unwrap();
        Self {
            default_loader,
            theme_loader,
            lang,
        }
    }

    pub fn t(&self, key: &str) -> String {
        self.theme_loader
            .as_ref()
            .and_then(|loader| {
                loader.lookup_no_default_fallback(
                    &self.lang,
                    key,
                    None as Option<&HashMap<&str, _>>,
                )
            })
            .unwrap_or(self.default_loader.lookup(&self.lang, key))
    }

    pub fn ta(&self, key: &str, args: &HashMap<Cow<'static, str>, FluentValue<'_>>) -> String {
        self.theme_loader
            .as_ref()
            .and_then(|loader| loader.lookup_no_default_fallback(&self.lang, key, Some(args)))
            .unwrap_or(self.default_loader.lookup_with_args(&self.lang, key, args))
    }

    pub fn language(&self) -> String {
        self.lang.to_string()
    }

    pub fn languages(&self) -> Vec<String> {
        let mut vec: Vec<_> = self
            .default_loader
            .locales()
            .map(|i| i.to_string())
            .collect();
        vec.sort_unstable();
        vec
    }
}

impl fmt::Debug for Locale {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Locale").field("lang", &self.lang).finish()
    }
}
