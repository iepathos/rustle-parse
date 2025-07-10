use crate::parser::error::ParseError;
use minijinja::{Environment, Value};
use std::collections::HashMap;

pub struct TemplateEngine {
    env: Environment<'static>,
}

impl TemplateEngine {
    pub fn new() -> Self {
        let mut env = Environment::new();

        // Add Ansible-compatible filters
        env.add_filter("default", filters::default_filter);
        env.add_filter("mandatory", filters::mandatory_filter);
        env.add_filter("regex_replace", filters::regex_replace_filter);
        env.add_filter("lower", filters::lower_filter);
        env.add_filter("upper", filters::upper_filter);
        env.add_filter("trim", filters::trim_filter);

        Self { env }
    }

    pub fn render_string(
        &self,
        template_str: &str,
        vars: &HashMap<String, serde_json::Value>,
    ) -> Result<String, ParseError> {
        let template =
            self.env
                .template_from_str(template_str)
                .map_err(|e| ParseError::Template {
                    file: "inline".to_string(),
                    line: 0,
                    message: e.to_string(),
                })?;

        let minijinja_vars: HashMap<String, Value> = vars
            .iter()
            .map(|(k, v)| (k.clone(), serde_json_to_minijinja_value(v)))
            .collect();

        template
            .render(&minijinja_vars)
            .map_err(|e| ParseError::Template {
                file: "inline".to_string(),
                line: 0,
                message: e.to_string(),
            })
    }

    pub fn render_value(
        &self,
        value: &serde_json::Value,
        vars: &HashMap<String, serde_json::Value>,
    ) -> Result<serde_json::Value, ParseError> {
        match value {
            serde_json::Value::String(s) => {
                if s.contains("{{") && s.contains("}}") {
                    let rendered = self.render_string(s, vars)?;
                    Ok(serde_json::Value::String(rendered))
                } else {
                    Ok(value.clone())
                }
            }
            serde_json::Value::Object(obj) => {
                let mut rendered_obj = serde_json::Map::new();
                for (k, v) in obj {
                    let rendered_value = self.render_value(v, vars)?;
                    rendered_obj.insert(k.clone(), rendered_value);
                }
                Ok(serde_json::Value::Object(rendered_obj))
            }
            serde_json::Value::Array(arr) => {
                let mut rendered_arr = Vec::new();
                for item in arr {
                    let rendered_item = self.render_value(item, vars)?;
                    rendered_arr.push(rendered_item);
                }
                Ok(serde_json::Value::Array(rendered_arr))
            }
            _ => Ok(value.clone()),
        }
    }
}

impl Default for TemplateEngine {
    fn default() -> Self {
        Self::new()
    }
}

fn serde_json_to_minijinja_value(value: &serde_json::Value) -> Value {
    match value {
        serde_json::Value::Null => Value::UNDEFINED,
        serde_json::Value::Bool(b) => Value::from(*b),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Value::from(i)
            } else if let Some(f) = n.as_f64() {
                Value::from(f)
            } else {
                Value::UNDEFINED
            }
        }
        serde_json::Value::String(s) => Value::from(s.clone()),
        serde_json::Value::Array(arr) => {
            let vec: Vec<Value> = arr.iter().map(serde_json_to_minijinja_value).collect();
            Value::from(vec)
        }
        serde_json::Value::Object(obj) => {
            let map: HashMap<String, Value> = obj
                .iter()
                .map(|(k, v)| (k.clone(), serde_json_to_minijinja_value(v)))
                .collect();
            Value::from_object(map)
        }
    }
}

mod filters {
    use minijinja::{Error, ErrorKind, Value};

    pub fn default_filter(value: Value, default: Value) -> Result<Value, Error> {
        if value.is_undefined() || value.is_none() {
            Ok(default)
        } else {
            Ok(value)
        }
    }

    pub fn mandatory_filter(value: Value, message: Option<Value>) -> Result<Value, Error> {
        if value.is_undefined() || value.is_none() {
            let msg = message
                .and_then(|v| v.as_str().map(|s| s.to_string()))
                .unwrap_or_else(|| "Mandatory variable not defined".to_string());
            Err(Error::new(ErrorKind::UndefinedError, msg))
        } else {
            Ok(value)
        }
    }

    pub fn regex_replace_filter(
        value: Value,
        pattern: Value,
        replacement: Value,
    ) -> Result<Value, Error> {
        let string = value.as_str().ok_or_else(|| {
            Error::new(
                ErrorKind::InvalidOperation,
                "regex_replace requires string input",
            )
        })?;
        let pattern_str = pattern.as_str().ok_or_else(|| {
            Error::new(
                ErrorKind::InvalidOperation,
                "regex_replace pattern must be string",
            )
        })?;
        let replacement_str = replacement.as_str().ok_or_else(|| {
            Error::new(
                ErrorKind::InvalidOperation,
                "regex_replace replacement must be string",
            )
        })?;

        let regex = regex::Regex::new(pattern_str)
            .map_err(|e| Error::new(ErrorKind::InvalidOperation, format!("Invalid regex: {e}")))?;

        let result = regex.replace_all(string, replacement_str);
        Ok(Value::from(result.to_string()))
    }

    pub fn lower_filter(value: Value) -> Result<Value, Error> {
        let string = value.as_str().ok_or_else(|| {
            Error::new(
                ErrorKind::InvalidOperation,
                "lower filter requires string input",
            )
        })?;
        Ok(Value::from(string.to_lowercase()))
    }

    pub fn upper_filter(value: Value) -> Result<Value, Error> {
        let string = value.as_str().ok_or_else(|| {
            Error::new(
                ErrorKind::InvalidOperation,
                "upper filter requires string input",
            )
        })?;
        Ok(Value::from(string.to_uppercase()))
    }

    pub fn trim_filter(value: Value) -> Result<Value, Error> {
        let string = value.as_str().ok_or_else(|| {
            Error::new(
                ErrorKind::InvalidOperation,
                "trim filter requires string input",
            )
        })?;
        Ok(Value::from(string.trim()))
    }
}
