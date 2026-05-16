use jekko_plugin_api::{
    CommandEntry, ExternalPluginManifest, JekkoPlugin, ModelPresetEntry, PluginError,
    PluginRegistry, PluginResult, ThemeEntry,
};
use std::collections::BTreeMap;

struct FakePlugin;

impl JekkoPlugin for FakePlugin {
    fn id(&self) -> &'static str {
        "fake.plugin"
    }

    fn register(&self, registry: &mut PluginRegistry) -> PluginResult<()> {
        registry.register_plugin(self.id())?;
        registry.register_theme(ThemeEntry {
            name: "fake-dark".into(),
            label: Some("Fake Dark".into()),
            mode: Some("dark".into()),
            colors: {
                let mut colors = BTreeMap::new();
                colors.insert("primary".into(), "#abcdef".into());
                colors
            },
        })?;
        registry.register_command(CommandEntry {
            id: "fake.open".into(),
            label: "Open Fake".into(),
            description: None,
            category: Some("fake".into()),
            keybind: None,
        })?;
        registry.register_model_preset(ModelPresetEntry {
            id: "fake.fast".into(),
            label: "Fake Fast".into(),
            provider: "anthropic".into(),
            model: "claude-opus-4-7".into(),
            options: serde_json::json!({ "temperature": 0.5 }),
        })?;
        registry.merge_config_defaults(serde_json::json!({ "fake_enabled": true }));
        Ok(())
    }
}

#[test]
fn registers_fake_plugin_state() {
    let mut registry = PluginRegistry::new();
    FakePlugin
        .register(&mut registry)
        .expect("register fake plugin");

    assert_eq!(registry.plugin_ids(), &["fake.plugin".to_string()]);
    assert!(registry.themes().contains_key("fake-dark"));
    assert!(registry.commands().contains_key("fake.open"));
    assert!(registry.model_presets().contains_key("fake.fast"));
    assert_eq!(
        registry
            .config_defaults()
            .get("fake_enabled")
            .and_then(|v| v.as_bool()),
        Some(true),
    );
}

#[test]
fn duplicate_plugin_id_rejected() {
    let mut registry = PluginRegistry::new();
    FakePlugin
        .register(&mut registry)
        .expect("first register ok");
    let err = FakePlugin
        .register(&mut registry)
        .expect_err("second register should fail");
    assert!(matches!(err, PluginError::DuplicatePlugin(id) if id == "fake.plugin"));
}

#[test]
fn duplicate_theme_rejected() {
    let mut registry = PluginRegistry::new();
    let theme = ThemeEntry {
        name: "shared".into(),
        label: None,
        mode: None,
        colors: BTreeMap::new(),
    };
    registry.register_theme(theme.clone()).expect("first theme");
    let err = registry
        .register_theme(theme)
        .expect_err("duplicate theme should fail");
    assert!(matches!(err, PluginError::DuplicateTheme(name) if name == "shared"));
}

#[test]
fn apply_manifest_writes_through() {
    let toml = r#"
id = "rt.demo"
version = "0.1.0"

config_defaults = { rt = "ok" }

[[themes]]
name = "rt-light"
mode = "light"

[[commands]]
id = "rt.open"
label = "RT Open"

[[model_presets]]
id = "rt.medium"
label = "RT Medium"
provider = "openai"
model = "gpt-4o"
"#;
    let manifest = ExternalPluginManifest::from_toml_str(toml).expect("parse");

    let mut registry = PluginRegistry::new();
    registry.apply_manifest(&manifest).expect("apply manifest");

    assert_eq!(registry.plugin_ids(), &["rt.demo".to_string()]);
    assert!(registry.themes().contains_key("rt-light"));
    assert!(registry.commands().contains_key("rt.open"));
    assert!(registry.model_presets().contains_key("rt.medium"));
    assert_eq!(
        registry
            .config_defaults()
            .get("rt")
            .and_then(|v| v.as_str()),
        Some("ok"),
    );
}

#[test]
fn merge_config_defaults_shallow_object_overlay() {
    let mut registry = PluginRegistry::new();
    registry.merge_config_defaults(serde_json::json!({ "a": 1, "b": 2 }));
    registry.merge_config_defaults(serde_json::json!({ "b": 99, "c": 3 }));

    assert_eq!(
        registry.config_defaults().get("a").and_then(|v| v.as_i64()),
        Some(1)
    );
    assert_eq!(
        registry.config_defaults().get("b").and_then(|v| v.as_i64()),
        Some(99)
    );
    assert_eq!(
        registry.config_defaults().get("c").and_then(|v| v.as_i64()),
        Some(3)
    );
}

#[test]
fn jekko_plugin_trait_is_object_safe() {
    let plugin: Box<dyn JekkoPlugin> = Box::new(FakePlugin);
    assert_eq!(plugin.id(), "fake.plugin");

    let mut registry = PluginRegistry::new();
    plugin
        .register(&mut registry)
        .expect("trait object register");
    assert!(!registry.plugin_ids().is_empty());
}
