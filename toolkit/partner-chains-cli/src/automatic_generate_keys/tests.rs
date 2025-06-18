use super::*;
use crate::tests::MockIOContext;

// Test helper function that creates mock session key type info for testing
fn create_mock_session_key_type_info(num_keys: usize) -> Vec<SessionKeyTypeInfo> {
    let mut info = Vec::new();
    
    match num_keys {
        2 => {
            info.push(SessionKeyTypeInfo {
                key_type_id: [b'a', b'u', b'r', b'a'],
                key_name: "Aura".to_string(),
                key_size: 32,
            });
            info.push(SessionKeyTypeInfo {
                key_type_id: [b'g', b'r', b'a', b'n'],
                key_name: "Grandpa".to_string(),
                key_size: 32,
            });
        }
        3 => {
            info.push(SessionKeyTypeInfo {
                key_type_id: [b'a', b'u', b'r', b'a'],
                key_name: "Aura".to_string(),
                key_size: 32,
            });
            info.push(SessionKeyTypeInfo {
                key_type_id: [b'g', b'r', b'a', b'n'],
                key_name: "Grandpa".to_string(),
                key_size: 32,
            });
            info.push(SessionKeyTypeInfo {
                key_type_id: [b'b', b'e', b'e', b'f'],
                key_name: "Beefy".to_string(),
                key_size: 32,
            });
        }
        4 => {
            info.push(SessionKeyTypeInfo {
                key_type_id: [b'a', b'u', b'r', b'a'],
                key_name: "Aura".to_string(),
                key_size: 32,
            });
            info.push(SessionKeyTypeInfo {
                key_type_id: [b'g', b'r', b'a', b'n'],
                key_name: "Grandpa".to_string(),
                key_size: 32,
            });
            info.push(SessionKeyTypeInfo {
                key_type_id: [b'b', b'e', b'e', b'f'],
                key_name: "Beefy".to_string(),
                key_size: 32,
            });
            info.push(SessionKeyTypeInfo {
                key_type_id: [b'i', b'm', b'o', b'n'],
                key_name: "ImOnline".to_string(),
                key_size: 32,
            });
        }
        5 => {
            info.push(SessionKeyTypeInfo {
                key_type_id: [b'a', b'u', b'r', b'a'],
                key_name: "Aura".to_string(),
                key_size: 32,
            });
            info.push(SessionKeyTypeInfo {
                key_type_id: [b'g', b'r', b'a', b'n'],
                key_name: "Grandpa".to_string(),
                key_size: 32,
            });
            info.push(SessionKeyTypeInfo {
                key_type_id: [b'b', b'e', b'e', b'f'],
                key_name: "Beefy".to_string(),
                key_size: 32,
            });
            info.push(SessionKeyTypeInfo {
                key_type_id: [b'i', b'm', b'o', b'n'],
                key_name: "ImOnline".to_string(),
                key_size: 32,
            });
            info.push(SessionKeyTypeInfo {
                key_type_id: [b'k', b'e', b'y', b'4'],
                key_name: "Key4".to_string(),
                key_size: 32,
            });
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
                info.push(SessionKeyTypeInfo {
                    key_type_id: key_id,
                    key_name,
                    key_size: 32,
                });
            }
        }
    }
    
    info
}

// Test helper function that uses the new type-aware parsing
fn parse_rotated_keys_for_test(keys_hex: &str, num_keys: usize) -> Result<SessionKeys> {
    let session_key_type_info = create_mock_session_key_type_info(num_keys);
    decode_session_keys_from_type_info(keys_hex, &session_key_type_info)
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
fn test_parse_keys_with_variable_sizes() {
    // Test with keys of different sizes
    let session_key_type_info = vec![
        SessionKeyTypeInfo {
            key_type_id: [b'a', b'u', b'r', b'a'],
            key_name: "Aura".to_string(),
            key_size: 32,
        },
        SessionKeyTypeInfo {
            key_type_id: [b'c', b'u', b's', b't'],
            key_name: "Custom".to_string(),
            key_size: 16, // Different size
        },
    ];
    
    // 48 bytes total (32 + 16)
    let keys_hex = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef\
                    fedcba9876543210fedcba9876543210";
    
    let result = decode_session_keys_from_type_info(keys_hex, &session_key_type_info).unwrap();
    
    assert_eq!(result.len(), 2);
    
    // Check Aura key (32 bytes)
    let (aura_type, aura_key) = &result[0];
    assert_eq!(*aura_type, [b'a', b'u', b'r', b'a']);
    assert_eq!(aura_key.len(), 32);
    assert_eq!(hex::encode(aura_key), "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef");
    
    // Check Custom key (16 bytes)
    let (custom_type, custom_key) = &result[1];
    assert_eq!(*custom_type, [b'c', b'u', b's', b't']);
    assert_eq!(custom_key.len(), 16);
    assert_eq!(hex::encode(custom_key), "fedcba9876543210fedcba9876543210");
}

#[test]
fn test_generate_key_type_id_from_name() {
    assert_eq!(generate_key_type_id_from_name("Aura"), [b'a', b'u', b'r', b'a']);
    assert_eq!(generate_key_type_id_from_name("Grandpa"), [b'g', b'r', b'a', b'n']);
    assert_eq!(generate_key_type_id_from_name("Beefy"), [b'b', b'e', b'e', b'f']);
    assert_eq!(generate_key_type_id_from_name("AB"), [b'a', b'b', 0, 0]);
    assert_eq!(generate_key_type_id_from_name("VeryLongName"), [b'v', b'e', b'r', b'y']);
}

#[test]
fn test_config_creation() {
    let context = MockIOContext::new();
    let config = AutomaticGenerateKeysConfig::load(&context, "http://example.com:9944".to_string());
    
    assert_eq!(config.node_url, "http://example.com:9944");
} 