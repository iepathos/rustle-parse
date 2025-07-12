use rustle_parse::parser::error::ParseError;
use rustle_parse::parser::template::TemplateEngine;
use std::collections::HashMap;

#[test]
fn test_default_filter_with_defined_value() {
    let engine = TemplateEngine::new();
    let mut vars = HashMap::new();
    vars.insert(
        "name".to_string(),
        serde_json::Value::String("Alice".to_string()),
    );

    let result = engine
        .render_string("{{ name | default('Bob') }}", &vars)
        .unwrap();
    assert_eq!(result, "Alice");
}

#[test]
fn test_default_filter_with_null_value() {
    let engine = TemplateEngine::new();
    let mut vars = HashMap::new();
    vars.insert("name".to_string(), serde_json::Value::Null);

    let result = engine
        .render_string("{{ name | default('Bob') }}", &vars)
        .unwrap();
    assert_eq!(result, "Bob");
}

#[test]
fn test_mandatory_filter_with_defined_value() {
    let engine = TemplateEngine::new();
    let mut vars = HashMap::new();
    vars.insert(
        "required_var".to_string(),
        serde_json::Value::String("value".to_string()),
    );

    let result = engine
        .render_string("{{ required_var | mandatory }}", &vars)
        .unwrap();
    assert_eq!(result, "value");
}

#[test]
fn test_mandatory_filter_with_undefined_value() {
    let engine = TemplateEngine::new();
    let vars = HashMap::new();

    let result = engine.render_string("{{ undefined_var | mandatory }}", &vars);
    assert!(result.is_err());
    match result.unwrap_err() {
        ParseError::Template { message, .. } => {
            assert!(message.contains("Mandatory variable not defined"));
        }
        _ => panic!("Expected Template error"),
    }
}

#[test]
fn test_mandatory_filter_with_custom_message() {
    let engine = TemplateEngine::new();
    let vars = HashMap::new();

    let result = engine.render_string(
        "{{ undefined_var | mandatory('Custom error message') }}",
        &vars,
    );
    assert!(result.is_err());
    match result.unwrap_err() {
        ParseError::Template { message, .. } => {
            assert!(message.contains("Custom error message"));
        }
        _ => panic!("Expected Template error"),
    }
}

#[test]
fn test_mandatory_filter_with_null_value() {
    let engine = TemplateEngine::new();
    let mut vars = HashMap::new();
    vars.insert("null_var".to_string(), serde_json::Value::Null);

    let result = engine.render_string("{{ null_var | mandatory }}", &vars);
    assert!(result.is_err());
}

#[test]
fn test_regex_replace_filter() {
    let engine = TemplateEngine::new();
    let mut vars = HashMap::new();
    vars.insert(
        "text".to_string(),
        serde_json::Value::String("hello world".to_string()),
    );

    let result = engine
        .render_string(r#"{{ text | regex_replace('world', 'universe') }}"#, &vars)
        .unwrap();
    assert_eq!(result, "hello universe");
}

#[test]
fn test_regex_replace_filter_with_regex_pattern() {
    let engine = TemplateEngine::new();
    let mut vars = HashMap::new();
    vars.insert(
        "text".to_string(),
        serde_json::Value::String("test123test456".to_string()),
    );

    let result = engine
        .render_string(r#"{{ text | regex_replace('\\d+', 'X') }}"#, &vars)
        .unwrap();
    assert_eq!(result, "testXtestX");
}

#[test]
fn test_regex_replace_filter_with_non_string_input() {
    let engine = TemplateEngine::new();
    let mut vars = HashMap::new();
    vars.insert(
        "number".to_string(),
        serde_json::Value::Number(serde_json::Number::from(123)),
    );

    let result = engine.render_string(r#"{{ number | regex_replace('1', 'X') }}"#, &vars);
    assert!(result.is_err());
    match result.unwrap_err() {
        ParseError::Template { message, .. } => {
            assert!(message.contains("regex_replace requires string input"));
        }
        _ => panic!("Expected Template error"),
    }
}

#[test]
fn test_regex_replace_filter_with_invalid_regex() {
    let engine = TemplateEngine::new();
    let mut vars = HashMap::new();
    vars.insert(
        "text".to_string(),
        serde_json::Value::String("hello".to_string()),
    );

    let result = engine.render_string(r#"{{ text | regex_replace('[', 'X') }}"#, &vars);
    assert!(result.is_err());
    match result.unwrap_err() {
        ParseError::Template { message, .. } => {
            assert!(message.contains("Invalid regex"));
        }
        _ => panic!("Expected Template error"),
    }
}

#[test]
fn test_lower_filter() {
    let engine = TemplateEngine::new();
    let mut vars = HashMap::new();
    vars.insert(
        "text".to_string(),
        serde_json::Value::String("HELLO WORLD".to_string()),
    );

    let result = engine.render_string("{{ text | lower }}", &vars).unwrap();
    assert_eq!(result, "hello world");
}

#[test]
fn test_lower_filter_with_mixed_case() {
    let engine = TemplateEngine::new();
    let mut vars = HashMap::new();
    vars.insert(
        "text".to_string(),
        serde_json::Value::String("HeLLo WoRLd".to_string()),
    );

    let result = engine.render_string("{{ text | lower }}", &vars).unwrap();
    assert_eq!(result, "hello world");
}

#[test]
fn test_lower_filter_with_non_string() {
    let engine = TemplateEngine::new();
    let mut vars = HashMap::new();
    vars.insert(
        "number".to_string(),
        serde_json::Value::Number(serde_json::Number::from(123)),
    );

    let result = engine.render_string("{{ number | lower }}", &vars);
    assert!(result.is_err());
    match result.unwrap_err() {
        ParseError::Template { message, .. } => {
            assert!(message.contains("lower filter requires string input"));
        }
        _ => panic!("Expected Template error"),
    }
}

#[test]
fn test_upper_filter() {
    let engine = TemplateEngine::new();
    let mut vars = HashMap::new();
    vars.insert(
        "text".to_string(),
        serde_json::Value::String("hello world".to_string()),
    );

    let result = engine.render_string("{{ text | upper }}", &vars).unwrap();
    assert_eq!(result, "HELLO WORLD");
}

#[test]
fn test_upper_filter_with_mixed_case() {
    let engine = TemplateEngine::new();
    let mut vars = HashMap::new();
    vars.insert(
        "text".to_string(),
        serde_json::Value::String("HeLLo WoRLd".to_string()),
    );

    let result = engine.render_string("{{ text | upper }}", &vars).unwrap();
    assert_eq!(result, "HELLO WORLD");
}

#[test]
fn test_upper_filter_with_non_string() {
    let engine = TemplateEngine::new();
    let mut vars = HashMap::new();
    vars.insert(
        "number".to_string(),
        serde_json::Value::Number(serde_json::Number::from(123)),
    );

    let result = engine.render_string("{{ number | upper }}", &vars);
    assert!(result.is_err());
    match result.unwrap_err() {
        ParseError::Template { message, .. } => {
            assert!(message.contains("upper filter requires string input"));
        }
        _ => panic!("Expected Template error"),
    }
}

#[test]
fn test_trim_filter() {
    let engine = TemplateEngine::new();
    let mut vars = HashMap::new();
    vars.insert(
        "text".to_string(),
        serde_json::Value::String("  hello world  ".to_string()),
    );

    let result = engine.render_string("{{ text | trim }}", &vars).unwrap();
    assert_eq!(result, "hello world");
}

#[test]
fn test_trim_filter_with_tabs_and_newlines() {
    let engine = TemplateEngine::new();
    let mut vars = HashMap::new();
    vars.insert(
        "text".to_string(),
        serde_json::Value::String("\t\nhello world\n\t".to_string()),
    );

    let result = engine.render_string("{{ text | trim }}", &vars).unwrap();
    assert_eq!(result, "hello world");
}

#[test]
fn test_trim_filter_with_non_string() {
    let engine = TemplateEngine::new();
    let mut vars = HashMap::new();
    vars.insert(
        "number".to_string(),
        serde_json::Value::Number(serde_json::Number::from(123)),
    );

    let result = engine.render_string("{{ number | trim }}", &vars);
    assert!(result.is_err());
    match result.unwrap_err() {
        ParseError::Template { message, .. } => {
            assert!(message.contains("trim filter requires string input"));
        }
        _ => panic!("Expected Template error"),
    }
}

#[test]
fn test_chained_filters() {
    let engine = TemplateEngine::new();
    let mut vars = HashMap::new();
    vars.insert(
        "text".to_string(),
        serde_json::Value::String("  HELLO WORLD  ".to_string()),
    );

    let result = engine
        .render_string("{{ text | trim | lower }}", &vars)
        .unwrap();
    assert_eq!(result, "hello world");
}

#[test]
fn test_multiple_filters_with_default() {
    let engine = TemplateEngine::new();
    let vars = HashMap::new();

    let result = engine
        .render_string("{{ undefined_var | default('HELLO') | lower }}", &vars)
        .unwrap();
    assert_eq!(result, "hello");
}

#[test]
fn test_template_with_complex_variables() {
    let engine = TemplateEngine::new();
    let mut vars = HashMap::new();
    vars.insert(
        "user".to_string(),
        serde_json::json!({
            "name": "Alice",
            "age": 30,
            "active": true
        }),
    );

    let result = engine
        .render_string("User: {{ user.name }}, Age: {{ user.age }}", &vars)
        .unwrap();
    assert_eq!(result, "User: Alice, Age: 30");
}

#[test]
fn test_template_with_arrays() {
    let engine = TemplateEngine::new();
    let mut vars = HashMap::new();
    vars.insert(
        "items".to_string(),
        serde_json::json!(["apple", "banana", "cherry"]),
    );

    let result = engine
        .render_string("First item: {{ items[0] }}", &vars)
        .unwrap();
    assert_eq!(result, "First item: apple");
}

#[test]
fn test_render_value_with_object() {
    let engine = TemplateEngine::new();
    let mut vars = HashMap::new();
    vars.insert(
        "env".to_string(),
        serde_json::Value::String("prod".to_string()),
    );

    let value = serde_json::json!({
        "database": {
            "host": "{{ env }}-db.example.com",
            "port": 5432
        }
    });

    let result = engine.render_value(&value, &vars).unwrap();

    assert_eq!(
        result["database"]["host"],
        serde_json::Value::String("prod-db.example.com".to_string())
    );
    assert_eq!(
        result["database"]["port"],
        serde_json::Value::Number(serde_json::Number::from(5432))
    );
}

#[test]
fn test_render_value_with_array() {
    let engine = TemplateEngine::new();
    let mut vars = HashMap::new();
    vars.insert(
        "env".to_string(),
        serde_json::Value::String("prod".to_string()),
    );

    let value = serde_json::json!([
        "{{ env }}-server1.example.com",
        "{{ env }}-server2.example.com",
        "static-server.example.com"
    ]);

    let result = engine.render_value(&value, &vars).unwrap();

    assert_eq!(
        result[0],
        serde_json::Value::String("prod-server1.example.com".to_string())
    );
    assert_eq!(
        result[1],
        serde_json::Value::String("prod-server2.example.com".to_string())
    );
    assert_eq!(
        result[2],
        serde_json::Value::String("static-server.example.com".to_string())
    );
}

#[test]
fn test_render_value_without_templates() {
    let engine = TemplateEngine::new();
    let vars = HashMap::new();

    let value = serde_json::json!({
        "static": "value",
        "number": 42,
        "boolean": true,
        "null_value": null
    });

    let result = engine.render_value(&value, &vars).unwrap();

    assert_eq!(result, value); // Should be unchanged
}

#[test]
fn test_render_value_with_nested_templates() {
    let engine = TemplateEngine::new();
    let mut vars = HashMap::new();
    vars.insert(
        "service".to_string(),
        serde_json::Value::String("api".to_string()),
    );
    vars.insert(
        "env".to_string(),
        serde_json::Value::String("prod".to_string()),
    );

    let value = serde_json::json!({
        "services": [
            {
                "name": "{{ service }}",
                "config": {
                    "host": "{{ service }}-{{ env }}.example.com",
                    "port": 8080
                }
            }
        ]
    });

    let result = engine.render_value(&value, &vars).unwrap();

    assert_eq!(
        result["services"][0]["name"],
        serde_json::Value::String("api".to_string())
    );
    assert_eq!(
        result["services"][0]["config"]["host"],
        serde_json::Value::String("api-prod.example.com".to_string())
    );
}

#[test]
fn test_invalid_template_syntax() {
    let engine = TemplateEngine::new();
    let vars = HashMap::new();

    let result = engine.render_string("{{ unclosed template", &vars);
    assert!(result.is_err());
    match result.unwrap_err() {
        ParseError::Template {
            file,
            line,
            message,
        } => {
            assert_eq!(file, "inline");
            assert_eq!(line, 0);
            assert!(message.contains("template") || message.contains("syntax"));
        }
        _ => panic!("Expected Template error"),
    }
}

#[test]
fn test_template_engine_default() {
    let engine = TemplateEngine::default();
    let mut vars = HashMap::new();
    vars.insert(
        "test".to_string(),
        serde_json::Value::String("value".to_string()),
    );

    let result = engine.render_string("{{ test }}", &vars).unwrap();
    assert_eq!(result, "value");
}

#[test]
fn test_empty_template_string() {
    let engine = TemplateEngine::new();
    let vars = HashMap::new();

    let result = engine.render_string("", &vars).unwrap();
    assert_eq!(result, "");
}

#[test]
fn test_template_with_special_characters() {
    let engine = TemplateEngine::new();
    let mut vars = HashMap::new();
    vars.insert(
        "special".to_string(),
        serde_json::Value::String("Hello, @#$%^&*()!".to_string()),
    );

    let result = engine.render_string("Value: {{ special }}", &vars).unwrap();
    assert_eq!(result, "Value: Hello, @#$%^&*()!");
}

#[test]
fn test_template_with_unicode() {
    let engine = TemplateEngine::new();
    let mut vars = HashMap::new();
    vars.insert(
        "unicode".to_string(),
        serde_json::Value::String("Hello 世界! 🌍".to_string()),
    );

    let result = engine.render_string("{{ unicode }}", &vars).unwrap();
    assert_eq!(result, "Hello 世界! 🌍");
}

#[test]
#[allow(clippy::approx_constant)]
fn test_json_value_conversion() {
    let engine = TemplateEngine::new();
    let mut vars = HashMap::new();
    vars.insert("bool_val".to_string(), serde_json::Value::Bool(true));
    vars.insert("null_val".to_string(), serde_json::Value::Null);
    vars.insert(
        "number_val".to_string(),
        serde_json::Value::Number(serde_json::Number::from(42)),
    );
    vars.insert(
        "float_val".to_string(),
        serde_json::Value::Number(serde_json::Number::from_f64(3.14).unwrap()),
    );

    let result = engine.render_string("{{ bool_val }}", &vars).unwrap();
    assert_eq!(result, "true");

    let result = engine.render_string("{{ number_val }}", &vars).unwrap();
    assert_eq!(result, "42");

    let result = engine.render_string("{{ float_val }}", &vars).unwrap();
    assert_eq!(result, "3.14");
}

#[test]
fn test_render_value_string_without_template_markers() {
    let engine = TemplateEngine::new();
    let mut vars = HashMap::new();
    vars.insert(
        "env".to_string(),
        serde_json::Value::String("prod".to_string()),
    );

    let value = serde_json::Value::String("static-string".to_string());
    let result = engine.render_value(&value, &vars).unwrap();

    assert_eq!(
        result,
        serde_json::Value::String("static-string".to_string())
    );
}

#[test]
fn test_template_error_propagation_in_render_value() {
    let engine = TemplateEngine::new();
    let vars = HashMap::new();

    let value = serde_json::json!({
        "config": {
            "invalid": "{{ undefined_var | mandatory }}"
        }
    });

    let result = engine.render_value(&value, &vars);
    assert!(result.is_err());
    match result.unwrap_err() {
        ParseError::Template { .. } => {
            // Expected
        }
        _ => panic!("Expected Template error"),
    }
}

#[test]
fn test_template_error_propagation_in_array() {
    let engine = TemplateEngine::new();
    let vars = HashMap::new();

    let value = serde_json::json!(["valid-string", "{{ undefined_var | mandatory }}"]);

    let result = engine.render_value(&value, &vars);
    assert!(result.is_err());
    match result.unwrap_err() {
        ParseError::Template { .. } => {
            // Expected
        }
        _ => panic!("Expected Template error"),
    }
}

#[test]
fn test_regex_replace_with_non_string_pattern() {
    let engine = TemplateEngine::new();
    let mut vars = HashMap::new();
    vars.insert(
        "text".to_string(),
        serde_json::Value::String("hello".to_string()),
    );
    vars.insert(
        "pattern".to_string(),
        serde_json::Value::Number(serde_json::Number::from(123)),
    );

    let result = engine.render_string("{{ text | regex_replace(pattern, 'X') }}", &vars);
    assert!(result.is_err());
    match result.unwrap_err() {
        ParseError::Template { message, .. } => {
            assert!(message.contains("regex_replace pattern must be string"));
        }
        _ => panic!("Expected Template error"),
    }
}

#[test]
fn test_regex_replace_with_non_string_replacement() {
    let engine = TemplateEngine::new();
    let mut vars = HashMap::new();
    vars.insert(
        "text".to_string(),
        serde_json::Value::String("hello".to_string()),
    );
    vars.insert(
        "replacement".to_string(),
        serde_json::Value::Number(serde_json::Number::from(123)),
    );

    let result = engine.render_string("{{ text | regex_replace('l', replacement) }}", &vars);
    assert!(result.is_err());
    match result.unwrap_err() {
        ParseError::Template { message, .. } => {
            assert!(message.contains("regex_replace replacement must be string"));
        }
        _ => panic!("Expected Template error"),
    }
}
