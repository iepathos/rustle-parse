use crate::parser::error::ParseError;
use once_cell::sync::Lazy;
use regex::Regex;

/// Represents a host pattern that can be expanded into individual host names.
#[derive(Debug, Clone)]
pub struct HostPattern {
    pub pattern: String,
    pub expanded: Vec<String>,
}

impl HostPattern {
    /// Create a new host pattern and expand it.
    pub fn new(pattern: &str) -> Result<Self, ParseError> {
        let mut host_pattern = Self {
            pattern: pattern.to_string(),
            expanded: Vec::new(),
        };
        host_pattern.expanded = host_pattern.expand()?;
        Ok(host_pattern)
    }

    /// Check if the pattern contains expandable elements.
    pub fn is_pattern(&self) -> bool {
        self.pattern.contains('[') && self.pattern.contains(']')
    }

    /// Check if the pattern appears to be attempting to use pattern syntax
    fn appears_to_be_pattern(&self) -> bool {
        self.pattern.contains('[') || self.pattern.contains(']')
    }

    /// Expand the host pattern into individual host names.
    pub fn expand(&self) -> Result<Vec<String>, ParseError> {
        // If it appears to be a pattern but isn't valid, validate and return error
        if self.appears_to_be_pattern() && !self.is_pattern() {
            self.validate_pattern()?;
        }

        if !self.is_pattern() {
            return Ok(vec![self.pattern.clone()]);
        }

        // Check for multiple bracket groups and handle them recursively
        let bracket_count = self.pattern.matches('[').count();
        if bracket_count > 1 {
            // Validate the pattern first, even for multiple brackets
            self.validate_pattern()?;
            return self.expand_multiple_patterns();
        }

        self.validate_single_pattern()?;

        let mut hosts = Vec::new();

        // Handle numeric ranges: web[01:05]
        if let Some(captures) = NUMERIC_PATTERN.captures(&self.pattern) {
            let prefix = &captures[1];
            let start_str = &captures[2];
            let end_str = &captures[3];
            let suffix = captures.get(4).map_or("", |m| m.as_str());

            let start: i32 = start_str
                .parse()
                .map_err(|_| ParseError::InvalidHostPattern {
                    pattern: self.pattern.clone(),
                    line: 0,
                    message: format!("Invalid start number: {start_str}"),
                })?;

            let end: i32 = end_str
                .parse()
                .map_err(|_| ParseError::InvalidHostPattern {
                    pattern: self.pattern.clone(),
                    line: 0,
                    message: format!("Invalid end number: {end_str}"),
                })?;

            if start > end {
                return Err(ParseError::InvalidHostPattern {
                    pattern: self.pattern.clone(),
                    line: 0,
                    message: format!("Start number {start} is greater than end number {end}"),
                });
            }

            // Check for zero-padding
            let zero_padded = start_str.starts_with('0') && start_str.len() > 1;
            let width = if zero_padded { start_str.len() } else { 0 };

            for i in start..=end {
                let formatted = if zero_padded {
                    format!("{prefix}{i:0width$}{suffix}")
                } else {
                    format!("{prefix}{i}{suffix}")
                };
                hosts.push(formatted);
            }
        }
        // Handle alphabetic ranges: db-[a:c]
        else if let Some(captures) = ALPHA_PATTERN.captures(&self.pattern) {
            let prefix = &captures[1];
            let start_char = captures[2].chars().next().unwrap();
            let end_char = captures[3].chars().next().unwrap();
            let suffix = captures.get(4).map_or("", |m| m.as_str());

            if start_char > end_char {
                return Err(ParseError::InvalidHostPattern {
                    pattern: self.pattern.clone(),
                    line: 0,
                    message: format!(
                        "Start character '{start_char}' is greater than end character '{end_char}'"
                    ),
                });
            }

            for c in start_char..=end_char {
                hosts.push(format!("{prefix}{c}{suffix}"));
            }
        }
        // Handle comma-separated lists: web[1,3,5]
        else if let Some(captures) = LIST_PATTERN.captures(&self.pattern) {
            let prefix = &captures[1];
            let list = &captures[2];
            let suffix = captures.get(3).map_or("", |m| m.as_str());

            for item in list.split(',') {
                let item = item.trim();
                if item.is_empty() {
                    continue;
                }
                hosts.push(format!("{prefix}{item}{suffix}"));
            }
        } else {
            return Err(ParseError::InvalidHostPattern {
                pattern: self.pattern.clone(),
                line: 0,
                message: "Unrecognized pattern format".to_string(),
            });
        }

        if hosts.is_empty() {
            return Err(ParseError::InvalidHostPattern {
                pattern: self.pattern.clone(),
                line: 0,
                message: "Pattern expansion resulted in no hosts".to_string(),
            });
        }

        // Limit pattern expansion to prevent resource exhaustion
        const MAX_HOSTS_PER_PATTERN: usize = 10000;
        if hosts.len() > MAX_HOSTS_PER_PATTERN {
            return Err(ParseError::InvalidHostPattern {
                pattern: self.pattern.clone(),
                line: 0,
                message: format!(
                    "Pattern expansion exceeds maximum limit of {MAX_HOSTS_PER_PATTERN} hosts"
                ),
            });
        }

        Ok(hosts)
    }

    /// Expand patterns with multiple bracket groups like node[a:c]-[1:2]
    fn expand_multiple_patterns(&self) -> Result<Vec<String>, ParseError> {
        // Find all bracket patterns in the string
        let bracket_regex = Regex::new(r"\[[^\]]+\]").unwrap();
        let mut pattern_indices = Vec::new();

        for mat in bracket_regex.find_iter(&self.pattern) {
            pattern_indices.push((mat.start(), mat.end()));
        }

        if pattern_indices.is_empty() {
            return Ok(vec![self.pattern.clone()]);
        }

        // Start with the base pattern and expand one bracket group at a time
        let mut current_patterns = vec![self.pattern.clone()];

        // Process from right to left to maintain correct indices
        for (start, end) in pattern_indices.into_iter().rev() {
            let mut new_patterns = Vec::new();

            for current_pattern in current_patterns {
                // Extract the bracket content
                let bracket_content = &current_pattern[start + 1..end - 1];
                let prefix = &current_pattern[..start];
                let suffix = &current_pattern[end..];

                // Determine the pattern type and expand
                let expansions: Vec<String> =
                    if bracket_content.contains(':') {
                        if bracket_content
                            .chars()
                            .all(|c| c.is_ascii_digit() || c == ':')
                        {
                            // Numeric pattern like [1:3]
                            let parts: Vec<&str> = bracket_content.split(':').collect();
                            if parts.len() == 2 {
                                let start_num: i32 = parts[0].parse().map_err(|_| {
                                    ParseError::InvalidHostPattern {
                                        pattern: self.pattern.clone(),
                                        line: 0,
                                        message: format!(
                                            "Invalid numeric pattern: {bracket_content}"
                                        ),
                                    }
                                })?;
                                let end_num: i32 = parts[1].parse().map_err(|_| {
                                    ParseError::InvalidHostPattern {
                                        pattern: self.pattern.clone(),
                                        line: 0,
                                        message: format!(
                                            "Invalid numeric pattern: {bracket_content}"
                                        ),
                                    }
                                })?;

                                let zero_padded = parts[0].starts_with('0') && parts[0].len() > 1;
                                let width = if zero_padded { parts[0].len() } else { 0 };

                                (start_num..=end_num)
                                    .map(|i| {
                                        if zero_padded {
                                            format!("{i:0width$}")
                                        } else {
                                            i.to_string()
                                        }
                                    })
                                    .collect()
                            } else {
                                return Err(ParseError::InvalidHostPattern {
                                    pattern: self.pattern.clone(),
                                    line: 0,
                                    message: format!("Invalid numeric range: {bracket_content}"),
                                });
                            }
                        } else {
                            // Alphabetic pattern like [a:c]
                            let parts: Vec<&str> = bracket_content.split(':').collect();
                            if parts.len() == 2 && parts[0].len() == 1 && parts[1].len() == 1 {
                                let start_char = parts[0].chars().next().unwrap();
                                let end_char = parts[1].chars().next().unwrap();
                                (start_char..=end_char).map(|c| c.to_string()).collect()
                            } else {
                                return Err(ParseError::InvalidHostPattern {
                                    pattern: self.pattern.clone(),
                                    line: 0,
                                    message: format!("Invalid alphabetic range: {bracket_content}"),
                                });
                            }
                        }
                    } else {
                        // List pattern like [red,blue,green]
                        bracket_content
                            .split(',')
                            .map(|s| s.trim().to_string())
                            .collect()
                    };

                // Create new patterns with each expansion
                for expansion in expansions {
                    new_patterns.push(format!("{prefix}{expansion}{suffix}"));
                }
            }

            current_patterns = new_patterns;
        }

        // Limit pattern expansion to prevent resource exhaustion
        const MAX_HOSTS_PER_PATTERN: usize = 10000;
        if current_patterns.len() > MAX_HOSTS_PER_PATTERN {
            return Err(ParseError::InvalidHostPattern {
                pattern: self.pattern.clone(),
                line: 0,
                message: format!(
                    "Pattern expansion exceeds maximum limit of {MAX_HOSTS_PER_PATTERN} hosts"
                ),
            });
        }

        Ok(current_patterns)
    }

    /// Validate pattern syntax before attempting expansion.
    fn validate_pattern(&self) -> Result<(), ParseError> {
        // Check for balanced brackets
        let open_brackets = self.pattern.matches('[').count();
        let close_brackets = self.pattern.matches(']').count();

        if open_brackets != close_brackets {
            return Err(ParseError::InvalidHostPattern {
                pattern: self.pattern.clone(),
                line: 0,
                message: "Unmatched brackets in host pattern".to_string(),
            });
        }

        if open_brackets == 0 {
            return Err(ParseError::InvalidHostPattern {
                pattern: self.pattern.clone(),
                line: 0,
                message: "No brackets found in pattern".to_string(),
            });
        }

        // Check for empty brackets
        if self.pattern.contains("[]") {
            return Err(ParseError::InvalidHostPattern {
                pattern: self.pattern.clone(),
                line: 0,
                message: "Empty brackets not allowed".to_string(),
            });
        }

        // Only allow multiple brackets if they're separated by non-bracket characters
        if open_brackets > 1 {
            // Check if multiple brackets are adjacent (not allowed)
            if self.pattern.contains("][") {
                return Err(ParseError::InvalidHostPattern {
                    pattern: self.pattern.clone(),
                    line: 0,
                    message: "Adjacent bracket patterns not supported".to_string(),
                });
            }
        }

        Ok(())
    }

    /// Validate single pattern syntax (legacy validation for single patterns).
    fn validate_single_pattern(&self) -> Result<(), ParseError> {
        // Check for balanced brackets
        let open_brackets = self.pattern.matches('[').count();
        let close_brackets = self.pattern.matches(']').count();

        if open_brackets != close_brackets {
            return Err(ParseError::InvalidHostPattern {
                pattern: self.pattern.clone(),
                line: 0,
                message: "Unmatched brackets in host pattern".to_string(),
            });
        }

        if open_brackets == 0 {
            return Err(ParseError::InvalidHostPattern {
                pattern: self.pattern.clone(),
                line: 0,
                message: "No brackets found in pattern".to_string(),
            });
        }

        if open_brackets > 1 {
            return Err(ParseError::InvalidHostPattern {
                pattern: self.pattern.clone(),
                line: 0,
                message: "Single pattern validation called on multi-pattern".to_string(),
            });
        }

        // Check for empty brackets
        if self.pattern.contains("[]") {
            return Err(ParseError::InvalidHostPattern {
                pattern: self.pattern.clone(),
                line: 0,
                message: "Empty brackets not allowed".to_string(),
            });
        }

        Ok(())
    }

    /// Create a lazy iterator for large patterns to avoid allocating all hosts at once.
    pub fn expand_lazy(&self) -> Result<HostPatternIterator, ParseError> {
        if !self.is_pattern() {
            return Ok(HostPatternIterator::Single {
                pattern: self.pattern.clone(),
                consumed: false,
            });
        }

        self.validate_pattern()?;

        // Handle numeric ranges
        if let Some(captures) = NUMERIC_PATTERN.captures(&self.pattern) {
            let prefix = captures[1].to_string();
            let start_str = &captures[2];
            let end_str = &captures[3];
            let suffix = captures.get(4).map_or("", |m| m.as_str()).to_string();

            let start: i32 = start_str
                .parse()
                .map_err(|_| ParseError::InvalidHostPattern {
                    pattern: self.pattern.clone(),
                    line: 0,
                    message: format!("Invalid start number: {start_str}"),
                })?;

            let end: i32 = end_str
                .parse()
                .map_err(|_| ParseError::InvalidHostPattern {
                    pattern: self.pattern.clone(),
                    line: 0,
                    message: format!("Invalid end number: {end_str}"),
                })?;

            let zero_padded = start_str.starts_with('0') && start_str.len() > 1;
            let width = if zero_padded { start_str.len() } else { 0 };

            return Ok(HostPatternIterator::Numeric {
                prefix,
                suffix,
                current: start,
                end,
                zero_padded,
                width,
            });
        }

        // For other patterns, fall back to immediate expansion
        let expanded = self.expand()?;
        Ok(HostPatternIterator::List {
            hosts: expanded.into_iter(),
        })
    }
}

/// Iterator for lazy host pattern expansion.
pub enum HostPatternIterator {
    Single {
        pattern: String,
        consumed: bool,
    },
    Numeric {
        prefix: String,
        suffix: String,
        current: i32,
        end: i32,
        zero_padded: bool,
        width: usize,
    },
    List {
        hosts: std::vec::IntoIter<String>,
    },
}

impl Iterator for HostPatternIterator {
    type Item = String;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            HostPatternIterator::Single { pattern, consumed } => {
                if *consumed {
                    None
                } else {
                    *consumed = true;
                    Some(pattern.clone())
                }
            }
            HostPatternIterator::Numeric {
                prefix,
                suffix,
                current,
                end,
                zero_padded,
                width,
            } => {
                if *current <= *end {
                    let result = if *zero_padded {
                        format!("{}{:0width$}{}", prefix, current, suffix, width = *width)
                    } else {
                        format!("{prefix}{current}{suffix}")
                    };
                    *current += 1;
                    Some(result)
                } else {
                    None
                }
            }
            HostPatternIterator::List { hosts } => hosts.next(),
        }
    }
}

// Compiled regex patterns for efficient matching
static NUMERIC_PATTERN: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^(.+)\[(\d+):(\d+)\](.*)$").unwrap());

static ALPHA_PATTERN: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^(.+)\[([a-z]):([a-z])\](.*)$").unwrap());

static LIST_PATTERN: Lazy<Regex> = Lazy::new(|| Regex::new(r"^(.+)\[([^:\]]+)\](.*)$").unwrap());

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_hostname() {
        let pattern = HostPattern::new("web1").unwrap();
        assert!(!pattern.is_pattern());
        assert_eq!(pattern.expanded, vec!["web1"]);
    }

    #[test]
    fn test_numeric_pattern_expansion() {
        let pattern = HostPattern::new("web[01:03]").unwrap();
        assert!(pattern.is_pattern());
        assert_eq!(pattern.expanded, vec!["web01", "web02", "web03"]);
    }

    #[test]
    fn test_numeric_pattern_no_zero_padding() {
        let pattern = HostPattern::new("web[1:3]").unwrap();
        assert_eq!(pattern.expanded, vec!["web1", "web2", "web3"]);
    }

    #[test]
    fn test_alphabetic_pattern_expansion() {
        let pattern = HostPattern::new("db-[a:c]").unwrap();
        assert_eq!(pattern.expanded, vec!["db-a", "db-b", "db-c"]);
    }

    #[test]
    fn test_list_pattern_expansion() {
        let pattern = HostPattern::new("web[1,3,5]").unwrap();
        assert_eq!(pattern.expanded, vec!["web1", "web3", "web5"]);
    }

    #[test]
    fn test_pattern_with_suffix() {
        let pattern = HostPattern::new("web[01:02].example.com").unwrap();
        assert_eq!(
            pattern.expanded,
            vec!["web01.example.com", "web02.example.com"]
        );
    }

    #[test]
    fn test_invalid_pattern_unmatched_brackets() {
        let result = HostPattern::new("web[01:03");
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ParseError::InvalidHostPattern { .. }
        ));
    }

    #[test]
    fn test_invalid_pattern_empty_brackets() {
        let result = HostPattern::new("web[]");
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_pattern_start_greater_than_end() {
        let result = HostPattern::new("web[05:01]");
        assert!(result.is_err());
    }

    #[test]
    fn test_lazy_iterator_numeric() {
        let pattern = HostPattern::new("web[1:3]").unwrap();
        let mut iter = pattern.expand_lazy().unwrap();
        assert_eq!(iter.next(), Some("web1".to_string()));
        assert_eq!(iter.next(), Some("web2".to_string()));
        assert_eq!(iter.next(), Some("web3".to_string()));
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn test_lazy_iterator_single() {
        let pattern = HostPattern::new("web1").unwrap();
        let mut iter = pattern.expand_lazy().unwrap();
        assert_eq!(iter.next(), Some("web1".to_string()));
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn test_lazy_iterator_alphabetic() {
        let pattern = HostPattern::new("db[a:c]").unwrap();
        let mut iter = pattern.expand_lazy().unwrap();
        assert_eq!(iter.next(), Some("dba".to_string()));
        assert_eq!(iter.next(), Some("dbb".to_string()));
        assert_eq!(iter.next(), Some("dbc".to_string()));
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn test_lazy_iterator_list() {
        let pattern = HostPattern::new("srv[web,db,cache]").unwrap();
        let mut iter = pattern.expand_lazy().unwrap();
        assert_eq!(iter.next(), Some("srvweb".to_string()));
        assert_eq!(iter.next(), Some("srvdb".to_string()));
        assert_eq!(iter.next(), Some("srvcache".to_string()));
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn test_appears_to_be_pattern() {
        let pattern1 = HostPattern::new("web1").unwrap();
        assert!(!pattern1.appears_to_be_pattern());

        // Test with valid patterns that contain brackets
        let pattern2 = HostPattern::new("web[1:2]").unwrap();
        assert!(pattern2.appears_to_be_pattern());

        let pattern3 = HostPattern::new("db[a:c]").unwrap();
        assert!(pattern3.appears_to_be_pattern());

        // Test edge case - single item in brackets (valid)
        let pattern4 = HostPattern::new("srv[prod]").unwrap();
        assert!(pattern4.appears_to_be_pattern());
    }

    #[test]
    fn test_multiple_patterns_basic() {
        let pattern = HostPattern::new("node[1:2]-[a:b]").unwrap();
        assert_eq!(pattern.expanded.len(), 4);
        assert!(pattern.expanded.contains(&"node1-a".to_string()));
        assert!(pattern.expanded.contains(&"node1-b".to_string()));
        assert!(pattern.expanded.contains(&"node2-a".to_string()));
        assert!(pattern.expanded.contains(&"node2-b".to_string()));
    }

    #[test]
    fn test_multiple_patterns_numeric() {
        let pattern = HostPattern::new("rack[1:2]-server[01:02]").unwrap();
        assert_eq!(pattern.expanded.len(), 4);
        assert!(pattern.expanded.contains(&"rack1-server01".to_string()));
        assert!(pattern.expanded.contains(&"rack1-server02".to_string()));
        assert!(pattern.expanded.contains(&"rack2-server01".to_string()));
        assert!(pattern.expanded.contains(&"rack2-server02".to_string()));
    }

    #[test]
    fn test_multiple_patterns_alphabetic() {
        let pattern = HostPattern::new("dc[a:b]-zone[x:y]").unwrap();
        assert_eq!(pattern.expanded.len(), 4);
        assert!(pattern.expanded.contains(&"dca-zonex".to_string()));
        assert!(pattern.expanded.contains(&"dca-zoney".to_string()));
        assert!(pattern.expanded.contains(&"dcb-zonex".to_string()));
        assert!(pattern.expanded.contains(&"dcb-zoney".to_string()));
    }

    #[test]
    fn test_multiple_patterns_list() {
        let pattern = HostPattern::new("env[prod,dev]-[web,db]").unwrap();
        assert_eq!(pattern.expanded.len(), 4);
        assert!(pattern.expanded.contains(&"envprod-web".to_string()));
        assert!(pattern.expanded.contains(&"envprod-db".to_string()));
        assert!(pattern.expanded.contains(&"envdev-web".to_string()));
        assert!(pattern.expanded.contains(&"envdev-db".to_string()));
    }

    #[test]
    fn test_multiple_patterns_mixed_types() {
        let pattern = HostPattern::new("host[1:2]-[a,b]-[x:y]").unwrap();
        assert_eq!(pattern.expanded.len(), 8); // 2 * 2 * 2
        assert!(pattern.expanded.contains(&"host1-a-x".to_string()));
        assert!(pattern.expanded.contains(&"host2-b-y".to_string()));
    }

    #[test]
    fn test_multiple_patterns_adjacent_brackets_error() {
        let result = HostPattern::new("host[1:2][a:b]");
        assert!(result.is_err());
        match result.unwrap_err() {
            ParseError::InvalidHostPattern { message, .. } => {
                assert!(message.contains("Adjacent bracket patterns"));
            }
            _ => panic!("Expected InvalidHostPattern error"),
        }
    }

    #[test]
    fn test_multiple_patterns_exceed_limit() {
        // Create a pattern that would expand to more than MAX_HOSTS_PER_PATTERN
        let result = HostPattern::new("host[1:100]-[1:100]-[1:100]"); // Would be 100*100*100 = 1,000,000
        assert!(result.is_err());
        match result.unwrap_err() {
            ParseError::InvalidHostPattern { message, .. } => {
                assert!(message.contains("exceeds maximum"));
            }
            _ => panic!("Expected InvalidHostPattern error"),
        }
    }

    #[test]
    fn test_validate_pattern_only_opening_bracket() {
        let result = HostPattern::new("host[");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_pattern_only_closing_bracket() {
        let result = HostPattern::new("host]");
        assert!(result.is_err());
        match result.unwrap_err() {
            ParseError::InvalidHostPattern { message, .. } => {
                assert!(message.contains("bracket"));
            }
            _ => panic!("Expected InvalidHostPattern error"),
        }
    }

    #[test]
    fn test_validate_pattern_multiple_validation_errors() {
        let result = HostPattern::new("host[]]extra[");
        assert!(result.is_err());
    }

    #[test]
    fn test_empty_expansion_error() {
        // Force an empty expansion by creating a custom test
        // This would need internal access, so we test indirectly
        let result = HostPattern::new("host[]");
        assert!(result.is_err());
    }

    #[test]
    fn test_numeric_pattern_invalid_start_format() {
        let result = HostPattern::new("host[abc:123]");
        assert!(result.is_err());
        // Simply check that it's an InvalidHostPattern error
        assert!(matches!(
            result.unwrap_err(),
            ParseError::InvalidHostPattern { .. }
        ));
    }

    #[test]
    fn test_numeric_pattern_invalid_end_format() {
        let result = HostPattern::new("host[123:xyz]");
        assert!(result.is_err());
        // Simply check that it's an InvalidHostPattern error
        assert!(matches!(
            result.unwrap_err(),
            ParseError::InvalidHostPattern { .. }
        ));
    }

    #[test]
    fn test_alphabetic_pattern_invalid_start() {
        let result = HostPattern::new("host[ab:c]");
        assert!(result.is_err());
    }

    #[test]
    fn test_alphabetic_pattern_invalid_end() {
        let result = HostPattern::new("host[a:cd]");
        assert!(result.is_err());
    }

    #[test]
    fn test_zero_padded_multiple_patterns() {
        let pattern = HostPattern::new("host[01:02]-[001:002]").unwrap();
        assert_eq!(pattern.expanded.len(), 4);
        assert!(pattern.expanded.contains(&"host01-001".to_string()));
        assert!(pattern.expanded.contains(&"host01-002".to_string()));
        assert!(pattern.expanded.contains(&"host02-001".to_string()));
        assert!(pattern.expanded.contains(&"host02-002".to_string()));
    }

    #[test]
    fn test_complex_multiple_pattern_validation() {
        // Test with complex pattern that has many groups
        let pattern = HostPattern::new("dc[1:2]-rack[01:02]-host[a:b]").unwrap();
        assert_eq!(pattern.expanded.len(), 8); // 2 * 2 * 2
    }

    #[test]
    fn test_pattern_expansion_limit_single() {
        // Test single pattern approaching the limit
        let result = HostPattern::new("host[1:10001]"); // Exceeds MAX_HOSTS_PER_PATTERN
        assert!(result.is_err());
        match result.unwrap_err() {
            ParseError::InvalidHostPattern { message, .. } => {
                assert!(message.contains("exceeds maximum"));
            }
            _ => panic!("Expected InvalidHostPattern error"),
        }
    }

    #[test]
    fn test_edge_case_single_item_in_brackets() {
        let pattern = HostPattern::new("host[a]").unwrap();
        assert_eq!(pattern.expanded, vec!["hosta"]);
    }

    #[test]
    fn test_pattern_with_special_chars() {
        let pattern = HostPattern::new("host-[1:2]_server").unwrap();
        assert_eq!(pattern.expanded, vec!["host-1_server", "host-2_server"]);
    }

    #[test]
    fn test_validate_single_pattern_with_multiple() {
        // Indirectly test validate_single_pattern error case
        let result = HostPattern::new("host[1:2]-[a:b]");
        // Should succeed as new() handles multiple patterns
        assert!(result.is_ok());
    }
}
