use serde_json::Value;

use crate::error::{InstallError, Result};

#[derive(Clone, Debug, PartialEq)]
pub struct JsoncDocument {
    value: Value,
}

impl JsoncDocument {
    pub fn parse(text: &str) -> Result<Self> {
        let normalized = normalize_jsonc(text)?;
        let value = serde_json::from_str(&normalized)
            .map_err(|error| InstallError::Setup(format!("invalid OpenCode JSON: {error}")))?;
        Ok(Self { value })
    }

    pub fn empty() -> Self {
        Self {
            value: Value::Object(serde_json::Map::new()),
        }
    }

    pub fn value_mut(&mut self) -> &mut Value {
        &mut self.value
    }

    pub fn value(&self) -> &Value {
        &self.value
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        let mut bytes = serde_json::to_vec_pretty(&self.value)
            .map_err(|error| InstallError::Setup(format!("serialize OpenCode JSON: {error}")))?;
        bytes.push(b'\n');
        Ok(bytes)
    }
}

pub fn normalize_jsonc(text: &str) -> Result<String> {
    let mut output = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    let mut in_string = false;
    let mut escaped = false;

    while let Some(character) = chars.next() {
        if in_string {
            output.push(character);
            if escaped {
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == '"' {
                in_string = false;
            }
            continue;
        }

        if character == '"' {
            in_string = true;
            output.push(character);
            continue;
        }
        if character == '/' && chars.peek() == Some(&'/') {
            chars.next();
            for comment_character in chars.by_ref() {
                if comment_character == '\n' {
                    output.push('\n');
                    break;
                }
            }
            continue;
        }
        if character == '/' && chars.peek() == Some(&'*') {
            chars.next();
            let mut closed = false;
            while let Some(comment_character) = chars.next() {
                if comment_character == '*' && chars.peek() == Some(&'/') {
                    chars.next();
                    closed = true;
                    break;
                }
                if comment_character == '\n' {
                    output.push('\n');
                }
            }
            if !closed {
                return Err(InstallError::Setup(
                    "unterminated OpenCode JSON comment".into(),
                ));
            }
            continue;
        }
        output.push(character);
    }

    if in_string {
        return Err(InstallError::Setup(
            "unterminated OpenCode JSON string".into(),
        ));
    }

    Ok(remove_trailing_commas(&output))
}

fn remove_trailing_commas(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut output = String::with_capacity(text.len());
    let mut in_string = false;
    let mut escaped = false;
    let mut index = 0;

    while index < chars.len() {
        let character = chars[index];
        if in_string {
            output.push(character);
            if escaped {
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == '"' {
                in_string = false;
            }
            index += 1;
            continue;
        }
        if character == '"' {
            in_string = true;
            output.push(character);
            index += 1;
            continue;
        }
        if character == ',' {
            let mut next = index + 1;
            while next < chars.len() && chars[next].is_whitespace() {
                next += 1;
            }
            if next < chars.len() && matches!(chars[next], '}' | ']') {
                index += 1;
                continue;
            }
        }
        output.push(character);
        index += 1;
    }
    output
}

#[cfg(test)]
mod tests {
    use super::{normalize_jsonc, JsoncDocument};

    #[test]
    fn comments_and_trailing_commas_are_removed_outside_strings() {
        let document = JsoncDocument::parse(
            r#"{
                // comment
                "url": "https://example.test//literal",
                "value": [1, 2,], /* block */
            }"#,
        )
        .unwrap();
        assert_eq!(document.value()["url"], "https://example.test//literal");
        assert_eq!(document.value()["value"], serde_json::json!([1, 2]));
    }

    #[test]
    fn unterminated_comments_are_rejected() {
        assert!(normalize_jsonc("{/* comment").is_err());
    }
}
