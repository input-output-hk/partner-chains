use super::*;
use crate::tests::*;

#[test]
fn test_parse_rotated_keys_basic() {
    // Sample 64-byte key (aura + grandpa)
    let keys_hex = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef\
                    fedcba9876543210fedcba9876543210fedcba9876543210fedcba9876543210";
    
    let result = parse_rotated_keys(keys_hex).unwrap();
    
    assert_eq!(result.len(), 2);
    
    // Check Aura key (first key)
    let (aura_type, aura_key) = &result[0];
    assert_eq!(*aura_type, AURA_KEY_TYPE);
    assert_eq!(hex::encode(aura_key), "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef");
    
    // Check Grandpa key (second key)
    let (gran_type, gran_key) = &result[1];
    assert_eq!(*gran_type, GRANDPA_KEY_TYPE);
    assert_eq!(hex::encode(gran_key), "fedcba9876543210fedcba9876543210fedcba9876543210fedcba9876543210");
}

#[test]
fn test_parse_rotated_keys_with_prefix() {
    // Test with 0x prefix
    let keys_hex = "0x0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef\
                     fedcba9876543210fedcba9876543210fedcba9876543210fedcba9876543210";
    
    let result = parse_rotated_keys(keys_hex).unwrap();
    
    assert_eq!(result.len(), 2);
    
    let (aura_type, aura_key) = &result[0];
    assert_eq!(*aura_type, AURA_KEY_TYPE);
    assert_eq!(hex::encode(aura_key), "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef");
    
    let (gran_type, gran_key) = &result[1];
    assert_eq!(*gran_type, GRANDPA_KEY_TYPE);
    assert_eq!(hex::encode(gran_key), "fedcba9876543210fedcba9876543210fedcba9876543210fedcba9876543210");
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
    
    assert_eq!(result.len(), 3);
    
    // Check Aura key
    let (aura_type, aura_key) = &result[0];
    assert_eq!(*aura_type, AURA_KEY_TYPE);
    assert_eq!(hex::encode(aura_key), "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef");
    
    // Check Grandpa key
    let (gran_type, gran_key) = &result[1];
    assert_eq!(*gran_type, GRANDPA_KEY_TYPE);
    assert_eq!(hex::encode(gran_key), "fedcba9876543210fedcba9876543210fedcba9876543210fedcba9876543210");
    
    // Check Beefy key
    let (beefy_type, beefy_key) = &result[2];
    assert_eq!(*beefy_type, BEEFY_KEY_TYPE);
    assert_eq!(hex::encode(beefy_key), "aaaaaaaaaaaaaaaa000000000000000000000000000000000000000000000000");
}

#[test]
fn test_parse_rotated_keys_with_all_keys() {
    // 128-byte key (aura + grandpa + beefy + imon)
    let keys_hex = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef\
                    fedcba9876543210fedcba9876543210fedcba9876543210fedcba9876543210\
                    aaaaaaaaaaaaaaaa000000000000000000000000000000000000000000000000\
                    bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
    
    let result = parse_rotated_keys(keys_hex).unwrap();
    
    assert_eq!(result.len(), 4);
    
    // Check Aura key
    let (aura_type, aura_key) = &result[0];
    assert_eq!(*aura_type, AURA_KEY_TYPE);
    assert_eq!(hex::encode(aura_key), "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef");
    
    // Check Grandpa key
    let (gran_type, gran_key) = &result[1];
    assert_eq!(*gran_type, GRANDPA_KEY_TYPE);
    assert_eq!(hex::encode(gran_key), "fedcba9876543210fedcba9876543210fedcba9876543210fedcba9876543210");
    
    // Check Beefy key
    let (beefy_type, beefy_key) = &result[2];
    assert_eq!(*beefy_type, BEEFY_KEY_TYPE);
    assert_eq!(hex::encode(beefy_key), "aaaaaaaaaaaaaaaa000000000000000000000000000000000000000000000000");
    
    // Check IMON key
    let (imon_type, imon_key) = &result[3];
    assert_eq!(*imon_type, IMON_KEY_TYPE);
    assert_eq!(hex::encode(imon_key), "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb");
}

#[test]
fn test_parse_rotated_keys_with_extra_keys() {
    // Test with more than 4 keys to verify generic handling
    let keys_hex = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef\
                    fedcba9876543210fedcba9876543210fedcba9876543210fedcba9876543210\
                    aaaaaaaaaaaaaaaa000000000000000000000000000000000000000000000000\
                    bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb\
                    cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";
    
    let result = parse_rotated_keys(keys_hex).unwrap();
    
    assert_eq!(result.len(), 5);
    
    // First 4 should use predefined key types
    assert_eq!(result[0].0, AURA_KEY_TYPE);
    assert_eq!(result[1].0, GRANDPA_KEY_TYPE);
    assert_eq!(result[2].0, BEEFY_KEY_TYPE);
    assert_eq!(result[3].0, IMON_KEY_TYPE);
    
    // 5th key should get a generic identifier
    let (generic_type, generic_key) = &result[4];
    assert_eq!(*generic_type, [b'k', b'e', b'y', 4]); // "key" + index 4
    assert_eq!(hex::encode(generic_key), "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc");
}

#[test]
fn test_key_type_constants() {
    // Verify our key type constants are correct
    assert_eq!(AURA_KEY_TYPE, *b"aura");
    assert_eq!(GRANDPA_KEY_TYPE, *b"gran");
    assert_eq!(BEEFY_KEY_TYPE, *b"beef");
    assert_eq!(IMON_KEY_TYPE, *b"imon");
}

#[test]
fn test_config_creation() {
    let mock_context = MockIOContext::new().with_expected_io(vec![
        MockIO::prompt("node base path", Some("./data"), "./my-test-data")
    ]);
    let url = "http://localhost:9944".to_string();
    
    let config = AutomaticGenerateKeysConfig::load(&mock_context, url.clone());
    
    assert_eq!(config.node_url, url);
    assert_eq!(config.substrate_node_base_path, "./my-test-data");
} 