use rustle_parse::parser::error::ParseError;
use rustle_parse::parser::inventory::patterns::HostPattern;

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
fn test_list_pattern_with_strings() {
    let pattern = HostPattern::new("queue[red,blue,green]").unwrap();
    assert_eq!(
        pattern.expanded,
        vec!["queuered", "queueblue", "queuegreen"]
    );
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
fn test_complex_pattern_with_prefix_and_suffix() {
    let pattern = HostPattern::new("prod-web[1:2]-server").unwrap();
    assert_eq!(
        pattern.expanded,
        vec!["prod-web1-server", "prod-web2-server"]
    );
}

#[test]
fn test_zero_padded_patterns() {
    let pattern = HostPattern::new("host[001:003]").unwrap();
    assert_eq!(pattern.expanded, vec!["host001", "host002", "host003"]);
}

#[test]
fn test_large_numeric_range() {
    let pattern = HostPattern::new("vm[10:12]").unwrap();
    assert_eq!(pattern.expanded, vec!["vm10", "vm11", "vm12"]);
}

#[test]
fn test_single_item_list() {
    let pattern = HostPattern::new("web[5]").unwrap();
    assert_eq!(pattern.expanded, vec!["web5"]);
}

#[test]
fn test_alphabetic_single_range() {
    let pattern = HostPattern::new("db-[a:a]").unwrap();
    assert_eq!(pattern.expanded, vec!["db-a"]);
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
    assert!(matches!(
        result.unwrap_err(),
        ParseError::InvalidHostPattern { .. }
    ));
}

#[test]
fn test_invalid_pattern_start_greater_than_end() {
    let result = HostPattern::new("web[05:01]");
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        ParseError::InvalidHostPattern { .. }
    ));
}

#[test]
fn test_invalid_pattern_alpha_start_greater_than_end() {
    let result = HostPattern::new("db-[z:a]");
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        ParseError::InvalidHostPattern { .. }
    ));
}

#[test]
fn test_invalid_pattern_nested_brackets() {
    let result = HostPattern::new("web[01:[02:03]]");
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        ParseError::InvalidHostPattern { .. }
    ));
}

#[test]
fn test_invalid_pattern_multiple_brackets() {
    let result = HostPattern::new("web[01:02][a:b]");
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        ParseError::InvalidHostPattern { .. }
    ));
}

#[test]
fn test_invalid_numeric_pattern() {
    let result = HostPattern::new("web[abc:def]");
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        ParseError::InvalidHostPattern { .. }
    ));
}

#[test]
fn test_pattern_expansion_limit() {
    // This should succeed (reasonable size)
    let pattern = HostPattern::new("web[01:10]").unwrap();
    assert_eq!(pattern.expanded.len(), 10);

    // This would exceed the maximum limit if we had a very large range
    // The actual limit is tested in the main parser tests
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
fn test_lazy_iterator_zero_padded() {
    let pattern = HostPattern::new("web[01:03]").unwrap();
    let mut iter = pattern.expand_lazy().unwrap();

    assert_eq!(iter.next(), Some("web01".to_string()));
    assert_eq!(iter.next(), Some("web02".to_string()));
    assert_eq!(iter.next(), Some("web03".to_string()));
    assert_eq!(iter.next(), None);
}

#[test]
fn test_lazy_iterator_with_prefix_suffix() {
    let pattern = HostPattern::new("prod-web[1:2]-server").unwrap();
    let mut iter = pattern.expand_lazy().unwrap();

    assert_eq!(iter.next(), Some("prod-web1-server".to_string()));
    assert_eq!(iter.next(), Some("prod-web2-server".to_string()));
    assert_eq!(iter.next(), None);
}

#[test]
fn test_alphabetic_edge_cases() {
    // Test full alphabet range
    let pattern = HostPattern::new("node-[a:z]").unwrap();
    assert_eq!(pattern.expanded.len(), 26);
    assert_eq!(pattern.expanded[0], "node-a");
    assert_eq!(pattern.expanded[25], "node-z");
}

#[test]
fn test_numeric_edge_cases() {
    // Test with leading zeros
    let pattern = HostPattern::new("vm[000:002]").unwrap();
    assert_eq!(pattern.expanded, vec!["vm000", "vm001", "vm002"]);

    // Test single digit
    let pattern = HostPattern::new("host[1:1]").unwrap();
    assert_eq!(pattern.expanded, vec!["host1"]);
}

#[test]
fn test_list_edge_cases() {
    // Test with spaces (should be handled)
    let pattern = HostPattern::new("web[1, 2, 3]").unwrap();
    assert_eq!(pattern.expanded, vec!["web1", "web2", "web3"]);

    // Test empty list items (should be ignored)
    let pattern = HostPattern::new("web[1,,3]").unwrap();
    assert_eq!(pattern.expanded, vec!["web1", "web3"]);

    // Test mixed alphanumeric
    let pattern = HostPattern::new("node[a1,b2,c3]").unwrap();
    assert_eq!(pattern.expanded, vec!["nodea1", "nodeb2", "nodec3"]);
}

#[test]
fn test_pattern_validation() {
    // Valid patterns should pass
    assert!(HostPattern::new("web[01:05]").is_ok());
    assert!(HostPattern::new("db-[a:c]").is_ok());
    assert!(HostPattern::new("cache[1,3,5]").is_ok());
    assert!(HostPattern::new("simple-host").is_ok());

    // Invalid patterns should fail
    assert!(HostPattern::new("web[").is_err());
    assert!(HostPattern::new("web]").is_err());
    assert!(HostPattern::new("web[01:03][a:b]").is_err());
    assert!(HostPattern::new("web[]").is_err());
}

#[test]
fn test_special_characters_in_patterns() {
    // Test with hyphens
    let pattern = HostPattern::new("web-server[01:02]").unwrap();
    assert_eq!(pattern.expanded, vec!["web-server01", "web-server02"]);

    // Test with underscores
    let pattern = HostPattern::new("web_server[1:2]").unwrap();
    assert_eq!(pattern.expanded, vec!["web_server1", "web_server2"]);

    // Test with dots (FQDN)
    let pattern = HostPattern::new("web[1:2].example.com").unwrap();
    assert_eq!(
        pattern.expanded,
        vec!["web1.example.com", "web2.example.com"]
    );
}

#[test]
fn test_large_ranges() {
    // Test larger numeric ranges
    let pattern = HostPattern::new("worker[001:100]").unwrap();
    assert_eq!(pattern.expanded.len(), 100);
    assert_eq!(pattern.expanded[0], "worker001");
    assert_eq!(pattern.expanded[99], "worker100");

    // Test that zero padding is preserved
    assert!(pattern.expanded[9].starts_with("worker0"));
}

#[test]
fn test_iterator_collection() {
    let pattern = HostPattern::new("web[1:5]").unwrap();
    let iter = pattern.expand_lazy().unwrap();
    let collected: Vec<String> = iter.collect();

    assert_eq!(collected, vec!["web1", "web2", "web3", "web4", "web5"]);
}

#[test]
fn test_pattern_type_detection() {
    let numeric_pattern = HostPattern::new("web[01:05]").unwrap();
    assert!(numeric_pattern.is_pattern());

    let alpha_pattern = HostPattern::new("db-[a:c]").unwrap();
    assert!(alpha_pattern.is_pattern());

    let list_pattern = HostPattern::new("cache[1,3,5]").unwrap();
    assert!(list_pattern.is_pattern());

    let simple_host = HostPattern::new("single-host").unwrap();
    assert!(!simple_host.is_pattern());
}
