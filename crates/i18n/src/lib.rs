//! Translation layer. Strings live in `locales/<lang>/main.ftl` and are embedded in the binary.
//!
//! Use `tr!("key")` or `tr!("key", name = value)`. The language can change at runtime with
//! [`set_language`]. A key missing in the active language falls back to English, and a key
//! missing everywhere renders as the key itself so the UI never panics; tests catch both.

use std::sync::{OnceLock, RwLock};

use fluent_bundle::concurrent::FluentBundle;
use fluent_bundle::{FluentArgs, FluentResource, FluentValue};
use unic_langid::LanguageIdentifier;

/// A UI language.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Language {
    English,
    Portuguese,
    Spanish,
    French,
    German,
    Italian,
}

impl Language {
    pub const ALL: [Language; 6] = [
        Language::English,
        Language::Portuguese,
        Language::Spanish,
        Language::French,
        Language::German,
        Language::Italian,
    ];

    /// BCP 47 tag, also the folder name under `locales/` and the value saved in settings.
    pub fn code(self) -> &'static str {
        match self {
            Language::English => "en-US",
            Language::Portuguese => "pt-BR",
            Language::Spanish => "es-ES",
            Language::French => "fr-FR",
            Language::German => "de-DE",
            Language::Italian => "it-IT",
        }
    }

    /// The language's name in itself. Never translated, so a lost user can find their language.
    pub fn autonym(self) -> &'static str {
        match self {
            Language::English => "English",
            Language::Portuguese => "Português (Brasil)",
            Language::Spanish => "Español",
            Language::French => "Français",
            Language::German => "Deutsch",
            Language::Italian => "Italiano",
        }
    }

    fn source(self) -> &'static str {
        match self {
            Language::English => include_str!("../locales/en-US/main.ftl"),
            Language::Portuguese => include_str!("../locales/pt-BR/main.ftl"),
            Language::Spanish => include_str!("../locales/es-ES/main.ftl"),
            Language::French => include_str!("../locales/fr-FR/main.ftl"),
            Language::German => include_str!("../locales/de-DE/main.ftl"),
            Language::Italian => include_str!("../locales/it-IT/main.ftl"),
        }
    }

    /// The raw catalog text, for tests and tooling.
    pub fn catalog(self) -> &'static str {
        self.source()
    }

    /// Exact code as saved in settings (`pt-BR`).
    pub fn from_code(code: &str) -> Option<Language> {
        Self::ALL.into_iter().find(|l| l.code() == code.trim())
    }

    /// Match an OS locale such as `pt_BR.UTF-8`, `de`, `fr-CA` or `en_GB`. Only the language part
    /// matters. Anything unknown gives `None`.
    pub fn from_locale(locale: &str) -> Option<Language> {
        let lang = locale
            .trim()
            .split(['_', '-', '.', '@'])
            .next()?
            .to_ascii_lowercase();
        match lang.as_str() {
            "en" => Some(Language::English),
            "pt" => Some(Language::Portuguese),
            "es" => Some(Language::Spanish),
            "fr" => Some(Language::French),
            "de" => Some(Language::German),
            "it" => Some(Language::Italian),
            _ => None,
        }
    }

    /// The language of the operating system, English when it is not one we ship.
    pub fn system() -> Language {
        sys_locale::get_locale()
            .and_then(|l| Language::from_locale(&l))
            .unwrap_or(Language::English)
    }
}

type Bundle = FluentBundle<FluentResource>;

fn build(language: Language) -> Bundle {
    let tag: LanguageIdentifier = language.code().parse().expect("valid language id");
    let mut bundle = FluentBundle::new_concurrent(vec![tag]);
    // Isolation marks would end up around numbers in the UI.
    bundle.set_use_isolating(false);
    let resource = FluentResource::try_new(language.source().to_string())
        .unwrap_or_else(|_| panic!("locales/{}/main.ftl must parse", language.code()));
    bundle
        .add_resource(resource)
        .expect("no duplicate message ids");
    bundle
}

struct State {
    language: Language,
    active: Bundle,
}

static ENGLISH: OnceLock<Bundle> = OnceLock::new();
static STATE: OnceLock<RwLock<State>> = OnceLock::new();

fn english() -> &'static Bundle {
    ENGLISH.get_or_init(|| build(Language::English))
}

fn state() -> &'static RwLock<State> {
    STATE.get_or_init(|| {
        RwLock::new(State {
            language: Language::English,
            active: build(Language::English),
        })
    })
}

/// Switch the UI language. Takes effect for every later `tr!` call.
pub fn set_language(language: Language) {
    let mut state = state().write().unwrap_or_else(|e| e.into_inner());
    if state.language != language {
        state.active = build(language);
        state.language = language;
    }
}

/// The language `tr!` currently uses.
pub fn current() -> Language {
    state().read().unwrap_or_else(|e| e.into_inner()).language
}

/// True if `key` has a message in English, the reference catalog.
pub fn has_key(key: &str) -> bool {
    english().has_message(key)
}

fn format(bundle: &Bundle, key: &str, args: &[(&str, String)]) -> Option<String> {
    let pattern = bundle.get_message(key)?.value()?;
    let mut fluent_args = FluentArgs::new();
    for (name, value) in args {
        fluent_args.set(*name, FluentValue::from(value.clone()));
    }
    let mut errors = Vec::new();
    Some(
        bundle
            .format_pattern(pattern, Some(&fluent_args), &mut errors)
            .into_owned(),
    )
}

/// Translate `key` with named arguments. Falls back to English, then to the key.
pub fn translate(key: &str, args: &[(&str, String)]) -> String {
    let state = state().read().unwrap_or_else(|e| e.into_inner());
    format(&state.active, key, args)
        .or_else(|| format(english(), key, args))
        .unwrap_or_else(|| key.to_string())
}

#[macro_export]
macro_rules! tr {
    ($key:expr) => {
        $crate::translate($key, &[])
    };
    ($key:expr, $($name:ident = $value:expr),+ $(,)?) => {
        $crate::translate($key, &[$((stringify!($name), ($value).to_string())),+])
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{BTreeMap, BTreeSet};
    use std::sync::Mutex;

    /// Tests that switch the language must not interleave.
    static LANGUAGE_LOCK: Mutex<()> = Mutex::new(());

    /// Message id -> the `$variables` its text uses.
    fn messages(source: &str) -> BTreeMap<String, BTreeSet<String>> {
        let mut map = BTreeMap::new();
        for line in source.lines() {
            let Some((key, text)) = line.split_once(" = ") else {
                continue;
            };
            let key = key.trim();
            if key.is_empty() || key.starts_with('#') {
                continue;
            }
            let mut vars = BTreeSet::new();
            let mut rest = text;
            while let Some(i) = rest.find("{ $") {
                let tail = &rest[i + 3..];
                let end = tail.find([' ', '}']).unwrap_or(tail.len());
                vars.insert(tail[..end].to_string());
                rest = &tail[end..];
            }
            map.insert(key.to_string(), vars);
        }
        map
    }

    #[test]
    fn plain_and_argument_messages() {
        let _guard = LANGUAGE_LOCK.lock().unwrap();
        set_language(Language::English);
        assert_eq!(tr!("toolbar-camera"), "Camera:");
        assert_eq!(
            tr!("status-live", width = 1280, height = 720, format = "MJPEG"),
            "Live: 1280x720, requested MJPEG"
        );
    }

    #[test]
    fn missing_key_falls_back_to_the_key() {
        assert_eq!(tr!("no-such-key"), "no-such-key");
        assert!(!has_key("no-such-key"));
    }

    #[test]
    fn every_catalog_parses() {
        for language in Language::ALL {
            let _ = build(language);
        }
    }

    #[test]
    fn every_language_has_the_english_keys_and_variables() {
        let english = messages(Language::English.catalog());
        assert!(english.len() > 100);
        for language in Language::ALL {
            let other = messages(language.catalog());
            for (key, vars) in &english {
                let found = other
                    .get(key)
                    .unwrap_or_else(|| panic!("{}: missing key {key}", language.code()));
                assert_eq!(
                    found,
                    vars,
                    "{}: variables differ in {key}",
                    language.code()
                );
            }
            for key in other.keys() {
                assert!(
                    english.contains_key(key),
                    "{}: extra key {key}",
                    language.code()
                );
            }
        }
    }

    #[test]
    fn translations_are_not_left_in_english() {
        // A catalog copied verbatim would pass the key test; make sure most text differs.
        let english = Language::English.catalog();
        for language in Language::ALL.into_iter().skip(1) {
            let same = messages(language.catalog())
                .keys()
                .filter(|key| {
                    let line = |src: &str| {
                        src.lines()
                            .find(|l| l.starts_with(&format!("{key} = ")))
                            .map(str::to_string)
                    };
                    line(english) == line(language.catalog())
                })
                .count();
            assert!(
                same < 40,
                "{}: {same} entries equal English",
                language.code()
            );
        }
    }

    #[test]
    fn switching_the_language_changes_output() {
        let _guard = LANGUAGE_LOCK.lock().unwrap();
        set_language(Language::Portuguese);
        assert_eq!(current(), Language::Portuguese);
        assert_eq!(tr!("toolbar-camera"), "Câmera:");
        set_language(Language::German);
        assert_ne!(tr!("toolbar-camera"), "Camera:");
        set_language(Language::English);
        assert_eq!(tr!("toolbar-camera"), "Camera:");
    }

    #[test]
    fn locale_names_map_to_languages() {
        assert_eq!(
            Language::from_locale("pt_BR.UTF-8"),
            Some(Language::Portuguese)
        );
        assert_eq!(Language::from_locale("pt-PT"), Some(Language::Portuguese));
        assert_eq!(Language::from_locale("de"), Some(Language::German));
        assert_eq!(Language::from_locale("FR_ca"), Some(Language::French));
        assert_eq!(Language::from_locale("en_GB"), Some(Language::English));
        assert_eq!(Language::from_locale("ja-JP"), None);
        assert_eq!(Language::from_locale(""), None);
    }

    #[test]
    fn codes_round_trip() {
        for language in Language::ALL {
            assert_eq!(Language::from_code(language.code()), Some(language));
        }
        assert_eq!(Language::from_code("xx"), None);
    }
}
