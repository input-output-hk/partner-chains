use super::*;
use crate::tests::*;

#[test]
fn test_parse_rotated_keys_basic() {
    // Sample 64-byte key (aura + grandpa)
    let keys_hex = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef\
                    fedcba9876543210fedcba9876543210fedcba9876543210fedcba9876543210";
    
    let result = parse_rotated_keys(keys_hex).unwrap();
    
    assert_eq!(result.aura, "0x0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef");
    assert_eq!(result.gran, "0xfedcba9876543210fedcba9876543210fedcba9876543210fedcba9876543210");
    assert!(result.beefy.is_none());
    assert!(result.imon.is_none());
}

#[test]
fn test_parse_rotated_keys_with_prefix() {
    // Test with 0x prefix
    let keys_hex = "0x0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef\
                     fedcba9876543210fedcba9876543210fedcba9876543210fedcba9876543210";
    
    let result = parse_rotated_keys(keys_hex).unwrap();
    
    assert_eq!(result.aura, "0x0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef");
    assert_eq!(result.gran, "0xfedcba9876543210fedcba9876543210fedcba9876543210fedcba9876543210");
}

#[test]
fn test_parse_rotated_keys_too_short() {
    let keys_hex = "0123456789abcdef"; // Too short
    
    let result = parse_rotated_keys(keys_hex);
    
    assert!(result.is_err());
}

#[test]
fn test_parse_rotated_keys_with_beefy() {
    // 96-byte key (aura + grandpa + beefy)
    let keys_hex = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef\
                    fedcba9876543210fedcba9876543210fedcba9876543210fedcba9876543210\
                    aaaaaaaaaaaaaaaa000000000000000000000000000000000000000000000000";
    
    let result = parse_rotated_keys(keys_hex).unwrap();
    
    assert_eq!(result.aura, "0x0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef");
    assert_eq!(result.gran, "0xfedcba9876543210fedcba9876543210fedcba9876543210fedcba9876543210");
    assert_eq!(result.beefy, Some("0xaaaaaaaaaaaaaaaa000000000000000000000000000000000000000000000000".to_string()));
    assert!(result.imon.is_none());
}

#[test]
fn test_parse_rotated_keys_with_all_keys() {
    // 128-byte key (aura + grandpa + beefy + imon)
    let keys_hex = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef\
                    fedcba9876543210fedcba9876543210fedcba9876543210fedcba9876543210\
                    aaaaaaaaaaaaaaaa000000000000000000000000000000000000000000000000\
                    bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
    
    let result = parse_rotated_keys(keys_hex).unwrap();
    
    assert_eq!(result.aura, "0x0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef");
    assert_eq!(result.gran, "0xfedcba9876543210fedcba9876543210fedcba9876543210fedcba9876543210");
    assert_eq!(result.beefy, Some("0xaaaaaaaaaaaaaaaa000000000000000000000000000000000000000000000000".to_string()));
    assert_eq!(result.imon, Some("0xbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".to_string()));
}

#[test]
fn test_verify_key_order_valid() {
    let keys = AutomaticGeneratedKeys {
        aura: "0x0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".to_string(),
        gran: "0xfedcba9876543210fedcba9876543210fedcba9876543210fedcba9876543210".to_string(),
        beefy: Some("0xaaaaaaaaaaaaaaaa000000000000000000000000000000000000000000000000".to_string()),
        imon: Some("0xbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".to_string()),
    };
    
    let mock_context = MockIOContext::new();
    let result = verify_key_order(&keys, &mock_context);
    
    assert!(result.is_ok());
}

#[test]
fn test_verify_key_order_invalid_aura() {
    let keys = AutomaticGeneratedKeys {
        aura: "invalid_key".to_string(), // Invalid format
        gran: "0xfedcba9876543210fedcba9876543210fedcba9876543210fedcba9876543210".to_string(),
        beefy: None,
        imon: None,
    };
    
    let mock_context = MockIOContext::new();
    let result = verify_key_order(&keys, &mock_context);
    
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Invalid Aura key format"));
}

#[test]
fn test_config_creation() {
    let mock_context = MockIOContext::new();
    let url = "http://localhost:9944".to_string();
    
    let config = AutomaticGenerateKeysConfig::load(&mock_context, url.clone());
    
    assert_eq!(config.node_url, url);
} 