use super::*;
use crate::tests::*;

// Test helper function that creates mock session keys info for testing
fn create_mock_session_keys_info(num_keys: usize) -> Vec<([u8; 4], String)> {
    let mut info = Vec::new();
    
    match num_keys {
        2 => {
            info.push(([b'a', b'u', b'r', b'a'], "Aura".to_string()));
            info.push(([b'g', b'r', b'a', b'n'], "Grandpa".to_string()));
        }
        3 => {
            info.push(([b'a', b'u', b'r', b'a'], "Aura".to_string()));
            info.push(([b'g', b'r', b'a', b'n'], "Grandpa".to_string()));
            info.push(([b'b', b'e', b'e', b'f'], "Beefy".to_string()));
        }
        4 => {
            info.push(([b'a', b'u', b'r', b'a'], "Aura".to_string()));
            info.push(([b'g', b'r', b'a', b'n'], "Grandpa".to_string()));
            info.push(([b'b', b'e', b'e', b'f'], "Beefy".to_string()));
            info.push(([b'i', b'm', b'o', b'n'], "ImOnline".to_string()));
        }
        5 => {
            info.push(([b'a', b'u', b'r', b'a'], "Aura".to_string()));
            info.push(([b'g', b'r', b'a', b'n'], "Grandpa".to_string()));
            info.push(([b'b', b'e', b'e', b'f'], "Beefy".to_string()));
            info.push(([b'i', b'm', b'o', b'n'], "ImOnline".to_string()));
            info.push(([b'k', b'e', b'y', b'4'], "Key4".to_string()));
        }
        _ => {
            // Generic fallback
            for i in 0..num_keys {
                let key_id = if i < 4 {
                    match i {
                        0 => [b'a', b'u', b'r', b'a'],
                        1 => [b'g', b'r', b'a', b'n'],
                        2 => [b'b', b'e', b'e', b'f'],
                        3 => [b'i', b'm', b'o', b'n'],
                        _ => unreachable!(),
                    }
                } else {
                    [b'k', b'e', b'y', (i as u8) + b'0']
                };
                let key_name = match i {
                    0 => "Aura".to_string(),
                    1 => "Grandpa".to_string(),
                    2 => "Beefy".to_string(),
                    3 => "ImOnline".to_string(),
                    _ => format!("Key{}", i),
                };
                info.push((key_id, key_name));
            }
        }
    }
    
    info
}

// Test helper function that uses the metadata-driven parsing
fn parse_rotated_keys_for_test(keys_hex: &str, num_keys: usize) -> Result<SessionKeys> {
    let session_keys_info = create_mock_session_keys_info(num_keys);
    parse_rotated_keys_with_metadata(keys_hex, &session_keys_info)
}

#[test]
fn test_parse_rotated_keys_basic() {
    // Sample 64-byte key (aura + grandpa)
    let keys_hex = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef\
                    fedcba9876543210fedcba9876543210fedcba9876543210fedcba9876543210";
    
    let result = parse_rotated_keys_for_test(keys_hex, 2).unwrap();
    
    assert_eq!(result.len(), 2);
    
    // Check Aura key (first key)
    let (aura_type, aura_key) = &result[0];
    assert_eq!(*aura_type, [b'a', b'u', b'r', b'a']);
    assert_eq!(hex::encode(aura_key), "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef");
    
    // Check Grandpa key (second key)
    let (gran_type, gran_key) = &result[1];
    assert_eq!(*gran_type, [b'g', b'r', b'a', b'n']);
    assert_eq!(hex::encode(gran_key), "fedcba9876543210fedcba9876543210fedcba9876543210fedcba9876543210");
}

#[test]
fn test_parse_rotated_keys_with_prefix() {
    // Test with 0x prefix
    let keys_hex = "0x0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef\
                     fedcba9876543210fedcba9876543210fedcba9876543210fedcba9876543210";
    
    let result = parse_rotated_keys_for_test(keys_hex, 2).unwrap();
    
    assert_eq!(result.len(), 2);
    
    let (aura_type, aura_key) = &result[0];
    assert_eq!(*aura_type, [b'a', b'u', b'r', b'a']);
    assert_eq!(hex::encode(aura_key), "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef");
    
    let (gran_type, gran_key) = &result[1];
    assert_eq!(*gran_type, [b'g', b'r', b'a', b'n']);
    assert_eq!(hex::encode(gran_key), "fedcba9876543210fedcba9876543210fedcba9876543210fedcba9876543210");
}

#[test]
fn test_parse_rotated_keys_too_short() {
    let keys_hex = "0123456789abcdef"; // Too short
    
    let result = parse_rotated_keys_for_test(keys_hex, 2);
    
    assert!(result.is_err());
}

#[test]
fn test_parse_rotated_keys_with_beefy() {
    // 96-byte key (aura + grandpa + beefy)
    let keys_hex = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef\
                    fedcba9876543210fedcba9876543210fedcba9876543210fedcba9876543210\
                    aaaaaaaaaaaaaaaa000000000000000000000000000000000000000000000000";
    
    let result = parse_rotated_keys_for_test(keys_hex, 3).unwrap();
    
    assert_eq!(result.len(), 3);
    
    // Check Aura key
    let (aura_type, aura_key) = &result[0];
    assert_eq!(*aura_type, [b'a', b'u', b'r', b'a']);
    assert_eq!(hex::encode(aura_key), "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef");
    
    // Check Grandpa key
    let (gran_type, gran_key) = &result[1];
    assert_eq!(*gran_type, [b'g', b'r', b'a', b'n']);
    assert_eq!(hex::encode(gran_key), "fedcba9876543210fedcba9876543210fedcba9876543210fedcba9876543210");
    
    // Check Beefy key
    let (beefy_type, beefy_key) = &result[2];
    assert_eq!(*beefy_type, [b'b', b'e', b'e', b'f']);
    assert_eq!(hex::encode(beefy_key), "aaaaaaaaaaaaaaaa000000000000000000000000000000000000000000000000");
}

#[test]
fn test_parse_rotated_keys_with_all_keys() {
    // 128-byte key (aura + grandpa + beefy + imon)
    let keys_hex = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef\
                    fedcba9876543210fedcba9876543210fedcba9876543210fedcba9876543210\
                    aaaaaaaaaaaaaaaa000000000000000000000000000000000000000000000000\
                    bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
    
    let result = parse_rotated_keys_for_test(keys_hex, 4).unwrap();
    
    assert_eq!(result.len(), 4);
    
    // Check Aura key
    let (aura_type, aura_key) = &result[0];
    assert_eq!(*aura_type, [b'a', b'u', b'r', b'a']);
    assert_eq!(hex::encode(aura_key), "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef");
    
    // Check Grandpa key
    let (gran_type, gran_key) = &result[1];
    assert_eq!(*gran_type, [b'g', b'r', b'a', b'n']);
    assert_eq!(hex::encode(gran_key), "fedcba9876543210fedcba9876543210fedcba9876543210fedcba9876543210");
    
    // Check Beefy key
    let (beefy_type, beefy_key) = &result[2];
    assert_eq!(*beefy_type, [b'b', b'e', b'e', b'f']);
    assert_eq!(hex::encode(beefy_key), "aaaaaaaaaaaaaaaa000000000000000000000000000000000000000000000000");
    
    // Check IMON key
    let (imon_type, imon_key) = &result[3];
    assert_eq!(*imon_type, [b'i', b'm', b'o', b'n']);
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
    
    let result = parse_rotated_keys_for_test(keys_hex, 5).unwrap();
    
    assert_eq!(result.len(), 5);
    
    // First 4 should use predefined key types
    assert_eq!(result[0].0, [b'a', b'u', b'r', b'a']);
    assert_eq!(result[1].0, [b'g', b'r', b'a', b'n']);
    assert_eq!(result[2].0, [b'b', b'e', b'e', b'f']);
    assert_eq!(result[3].0, [b'i', b'm', b'o', b'n']);
    
    // 5th key should get a generic identifier
    let (generic_type, generic_key) = &result[4];
    assert_eq!(*generic_type, [b'k', b'e', b'y', b'4']);
    assert_eq!(hex::encode(generic_key), "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc");
}

#[test]
fn test_parse_keys_from_bytes() {
    let key_bytes = hex::decode("0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef\
                                fedcba9876543210fedcba9876543210fedcba9876543210fedcba9876543210").unwrap();
    
    let session_keys_info = vec![
        ([b'a', b'u', b'r', b'a'], "Aura".to_string()),
        ([b'g', b'r', b'a', b'n'], "Grandpa".to_string()),
    ];
    
    let result = decode_session_keys_from_key_info(&key_bytes, &session_keys_info).unwrap();
    
    assert_eq!(result.len(), 2);
    assert_eq!(result[0].0, [b'a', b'u', b'r', b'a']);
    assert_eq!(result[1].0, [b'g', b'r', b'a', b'n']);
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