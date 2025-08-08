use super::*;
use anyhow::Result;
use parity_scale_codec::Encode;

#[tokio::test]
async fn test_parse_decoded_keys_response_modern_format() -> Result<()> {
	// Test parsing modern Polkadot SDK format: Option<Vec<(Vec<u8>, u32)>>
	let key_data = vec![
		(b"aura_public_key".to_vec(), 0x61757261u32), // 'aura' as u32
		(b"grandpa_public_key".to_vec(), 0x6772616eu32), // 'gran' as u32
	];
	let encoded = Some(key_data).encode();
	
	let result = parse_decoded_keys_response(&encoded)?;
	
	assert_eq!(result.len(), 2);
	assert_eq!(result[0].0, 0x61757261u32.to_le_bytes().to_vec());
	assert_eq!(result[0].1, b"aura_public_key".to_vec());
	assert_eq!(result[1].0, 0x6772616eu32.to_le_bytes().to_vec());
	assert_eq!(result[1].1, b"grandpa_public_key".to_vec());
	
	Ok(())
}

#[tokio::test]
async fn test_parse_decoded_keys_response_legacy_format() -> Result<()> {
	// Test parsing legacy format: Vec<(Vec<u8>, Vec<u8>)>
	let key_data = vec![
		(b"aura".to_vec(), b"aura_public_key".to_vec()),
		(b"gran".to_vec(), b"grandpa_public_key".to_vec()),
	];
	let encoded = key_data.encode();
	
	let result = parse_decoded_keys_response(&encoded)?;
	
	assert_eq!(result.len(), 2);
	assert_eq!(result[0].0, b"aura".to_vec());
	assert_eq!(result[0].1, b"aura_public_key".to_vec());
	assert_eq!(result[1].0, b"gran".to_vec());
	assert_eq!(result[1].1, b"grandpa_public_key".to_vec());
	
	Ok(())
}

#[tokio::test]
async fn test_parse_decoded_keys_response_empty() -> Result<()> {
	// Test parsing empty response: Option<Vec<(Vec<u8>, u32)>> = None
	let encoded = Option::<Vec<(Vec<u8>, u32)>>::None.encode();
	
	let result = parse_decoded_keys_response(&encoded)?;
	
	assert_eq!(result.len(), 0);
	
	Ok(())
}

#[tokio::test]
async fn test_parse_decoded_keys_response_invalid() {
	// Test parsing invalid data
	let invalid_data = b"invalid_data";
	
	let result = parse_decoded_keys_response(invalid_data);
	
	assert!(result.is_err());
	assert!(result.unwrap_err().to_string().contains("Failed to SCALE decode keys"));
}

// The core SCALE decoding functionality is thoroughly tested above.
// The MockIO system in this codebase doesn't provide comprehensive file operation mocking,
// so more detailed integration tests would require a separate test framework or actual
// HTTP server setup for testing the full RPC workflow. 