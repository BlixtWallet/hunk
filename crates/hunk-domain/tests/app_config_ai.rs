use hunk_domain::config::AppConfig;

#[test]
fn ai_config_defaults_keep_awake_off() {
    let config = AppConfig::default();

    assert!(!config.ai.prevent_idle_sleep);
}

#[test]
fn ai_config_parses_keep_awake_when_present() {
    let raw = r#"
[ai]
prevent_idle_sleep = true
"#;

    let config: AppConfig = toml::from_str(raw).expect("AI config should parse");

    assert!(config.ai.prevent_idle_sleep);
}

#[test]
fn ai_config_round_trips_to_toml_group() {
    let mut config = AppConfig::default();
    config.ai.prevent_idle_sleep = true;

    let serialized = toml::to_string_pretty(&config).expect("config should serialize");

    assert!(serialized.contains("[ai]"));
    assert!(serialized.contains("prevent_idle_sleep = true"));
}
