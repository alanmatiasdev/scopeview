//! Guards the i18n layer: every key the UI asks for exists, and no catalog entry is dead.

const SOURCES: [&str; 7] = [
    include_str!("app.rs"),
    include_str!("app/help.rs"),
    include_str!("app/recent.rs"),
    include_str!("app/recording.rs"),
    include_str!("app/windows.rs"),
    include_str!("paint.rs"),
    include_str!("shortcuts.rs"),
];
const CATALOG: &str = include_str!("../../i18n/locales/en-US/main.ftl");

fn catalog_keys() -> Vec<&'static str> {
    CATALOG
        .lines()
        .filter_map(|line| line.split_once(" = "))
        .map(|(key, _)| key.trim())
        .filter(|key| !key.is_empty() && !key.starts_with('#'))
        .collect()
}

#[test]
fn every_tr_literal_exists_in_the_catalog() {
    let mut checked = 0;
    for source in SOURCES {
        for part in source.split("tr!(\"").skip(1) {
            let key = part.split('"').next().unwrap();
            assert!(scopeview_i18n::has_key(key), "missing i18n key: {key}");
            checked += 1;
        }
    }
    assert!(checked > 20, "scan found only {checked} keys");
}

#[test]
fn every_catalog_entry_is_used() {
    for key in catalog_keys() {
        let quoted = format!("\"{key}\"");
        assert!(
            SOURCES.iter().any(|s| s.contains(&quoted)),
            "unused i18n key: {key}"
        );
    }
}
