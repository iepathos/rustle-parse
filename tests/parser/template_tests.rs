use rustle_parse::parser::template::TemplateEngine;
use std::collections::HashMap;

#[test]
fn test_simple_template_rendering() {
    let engine = TemplateEngine::new();
    let mut vars = HashMap::new();
    vars.insert(
        "name".to_string(),
        serde_json::Value::String("world".to_string()),
    );

    let result = engine.render_string("Hello {{ name }}!", &vars);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "Hello world!");
}

#[test]
fn test_template_with_default_filter() {
    let engine = TemplateEngine::new();
    let vars = HashMap::new(); // Empty vars

    let result = engine.render_string("Hello {{ name | default('stranger') }}!", &vars);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "Hello stranger!");
}

#[test]
fn test_template_value_rendering() {
    let engine = TemplateEngine::new();
    let mut vars = HashMap::new();
    vars.insert(
        "user".to_string(),
        serde_json::Value::String("admin".to_string()),
    );

    let input_value = serde_json::json!({
        "username": "{{ user }}",
        "static_field": "unchanged"
    });

    let result = engine.render_value(&input_value, &vars);
    assert!(result.is_ok());

    let rendered = result.unwrap();
    assert_eq!(rendered["username"], "admin");
    assert_eq!(rendered["static_field"], "unchanged");
}

#[test]
fn test_template_with_undefined_variable() {
    let engine = TemplateEngine::new();
    let vars = HashMap::new();

    let result = engine.render_string("Hello {{ undefined_var }}!", &vars);
    // This should succeed but render as empty/undefined behavior
    // depending on minijinja's default behavior
    assert!(result.is_ok());
}

#[test]
fn test_template_filters() {
    let engine = TemplateEngine::new();
    let mut vars = HashMap::new();
    vars.insert(
        "text".to_string(),
        serde_json::Value::String("  Hello World  ".to_string()),
    );

    // Test lower filter
    let result = engine.render_string("{{ text | lower | trim }}", &vars);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "hello world");

    // Test upper filter
    let result = engine.render_string("{{ text | upper | trim }}", &vars);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "HELLO WORLD");
}
