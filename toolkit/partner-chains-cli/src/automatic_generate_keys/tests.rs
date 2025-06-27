use super::*;
use crate::tests::{MockIO, MockIOContext};
use mockall::{mock, predicate::*};
use subxt::{
	OnlineClient, SubstrateConfig,
	dynamic::Value,
	metadata::{Metadata, TypeDef},
};
use tokio;

mock! {
	pub OnlineClient {
		async fn from_url(url: &str) -> Result<Self, subxt::Error>;
		fn metadata(&self) -> Metadata;
	}
}

mock! {
	pub ReqwestClient {
		async fn post(&self, url: &str) -> MockReqwestRequestBuilder;
	}
}

mock! {
	pub ReqwestRequestBuilder {
		fn header(self, key: &'static str, value: &'static str) -> Self;
		fn json(self, value: &JsonRpcRequest) -> Self;
		async fn send(self) -> Result<reqwest::Response, reqwest::Error>;
	}
}

mock! {
	pub ReqwestResponse {
		async fn json<T: serde::de::DeserializeOwned>(self) -> Result<T, reqwest::Error>;
	}
}

#[test]
fn test_config_creation() {
	let mock_context = MockIOContext::new();
	let url = "http://example.com:9933".to_string();

	let config = AutomaticGenerateKeysConfig::load(&mock_context, url);
	assert_eq!(config.node_url, "http://example.com:9933");
}

#[tokio::test]
async fn test_generate_keys_with_valid_response() {
	let mock_context = MockIOContext::new().with_expected_io(vec![
        MockIO::eprint("🔑 Generating session keys via RPC..."),
        MockIO::eprint("✅ Generated session keys: 0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef5678901234567890ab5678901234567890ab5678901234567890ab5678901234"),
        MockIO::eprint("🔍 Fetching session key types from metadata..."),
        MockIO::eprint("📝 Found key types: [\"Aura\", \"Grandpa\"]"),
        MockIO::eprint("🔍 Parsing session keys..."),
        MockIO::eprint("  📝 Parsed Aura key: 0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef"),
        MockIO::eprint("  📝 Parsed Grandpa key: 0x5678901234567890ab5678901234567890ab5678901234567890ab5678901234"),
        MockIO::eprint("✅ Successfully parsed 2 session keys"),
        MockIO::write_file("session_keys.json"),
        MockIO::eprint("💾 Session keys saved to session_keys.json"),
        MockIO::eprint("🔑 Generated session keys:"),
        MockIO::print(""),
    ]);

	let keys_hex = "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef5678901234567890ab5678901234567890ab5678901234567890ab5678901234";
	let mut mock_client = MockReqwestClient::new();
	mock_client
		.expect_post()
		.once()
		.with(eq("http://localhost:9933"))
		.returning(|_| {
			let mut request_builder = MockReqwestRequestBuilder::new();
			request_builder
				.expect_header()
				.once()
				.with(eq("Content-Type"), eq("application/json"))
				.returning(|_, _| MockReqwestRequestBuilder::new());
			request_builder.expect_json().once().returning(|_| {
				let mut response = MockReqwestResponse::new();
				response.expect_json::<JsonRpcResponse>().once().returning(move |_| {
					Ok(JsonRpcResponse {
						jsonrpc: "2.0".to_string(),
						result: Some(keys_hex.to_string()),
						error: None,
						id: 1,
					})
				});
				MockReqwestRequestBuilder::new()
			});
			request_builder
		});

	let mut mock_subxt_client = MockOnlineClient::new();
	mock_subxt_client
		.expect_from_url()
		.once()
		.with(eq("http://localhost:9933"))
		.returning(|_| Ok(MockOnlineClient::new()));
	mock_subxt_client.expect_metadata().once().returning(|| {
		let mut metadata = Metadata::new();
		metadata.types_mut().register_type(
			0,
			scale_type_resolver::Type {
				path: scale_type_resolver::TypePath::from_segments([
					"pallet_session",
					"SessionKeys",
				])
				.unwrap(),
				type_def: TypeDef::Tuple(scale_type_resolver::Tuple::new(vec![1, 2])),
				docs: vec![],
			},
		);
		metadata.types_mut().register_type(
			1,
			scale_type_resolver::Type {
				path: scale_type_resolver::TypePath::from_segments([
					"sp_consensus_aura",
					"sr25519",
					"app",
					"Public",
				])
				.unwrap(),
				type_def: TypeDef::Composite(scale_type_resolver::Composite::new(vec![])),
				docs: vec![],
			},
		);
		metadata.types_mut().register_type(
			2,
			scale_type_resolver::Type {
				path: scale_type_resolver::TypePath::from_segments([
					"sp_finality_grandpa",
					"app",
					"Public",
				])
				.unwrap(),
				type_def: TypeDef::Composite(scale_type_resolver::Composite::new(vec![])),
				docs: vec![],
			},
		);
		metadata
	});

	let config = AutomaticGenerateKeysConfig { node_url: "http://localhost:9933".to_string() };

	// Mock reqwest::Client creation
	let original_client = reqwest::Client::new;
	let _mock = mockall::mock! {
		pub Client {}
		impl reqwest::ClientBuilder for Client {
			fn build(&self) -> Result<reqwest::Client, reqwest::Error>;
		}
	};
	let mut mock_client_builder = MockClient::new();
	mock_client_builder
		.expect_build()
		.once()
		.returning(move || Ok(mock_client.clone()));

	let result = generate_keys_via_rpc(&config, &mock_context).await;
	assert!(result.is_ok());
}

#[tokio::test]
async fn test_generate_keys_with_rpc_error() {
	let mock_context = MockIOContext::new()
		.with_expected_io(vec![MockIO::eprint("🔑 Generating session keys via RPC...")]);

	let mut mock_client = MockReqwestClient::new();
	mock_client
		.expect_post()
		.once()
		.with(eq("http://localhost:9933"))
		.returning(|_| {
			let mut request_builder = MockReqwestRequestBuilder::new();
			request_builder
				.expect_header()
				.once()
				.with(eq("Content-Type"), eq("application/json"))
				.returning(|_, _| MockReqwestRequestBuilder::new());
			request_builder.expect_json().once().returning(|_| {
				let mut response = MockReqwestResponse::new();
				response.expect_json::<JsonRpcResponse>().once().returning(|_| {
					Ok(JsonRpcResponse {
						jsonrpc: "2.0".to_string(),
						result: None,
						error: Some(JsonRpcError {
							code: -32601,
							message: "Method not found: unsafe".to_string(),
						}),
						id: 1,
					})
				});
				MockReqwestRequestBuilder::new()
			});
			request_builder
		});

	let config = AutomaticGenerateKeysConfig { node_url: "http://localhost:9933".to_string() };

	let original_client = reqwest::Client::new;
	let _mock = mockall::mock! {
		pub Client {}
		impl reqwest::ClientBuilder for Client {
			fn build(&self) -> Result<reqwest::Client, reqwest::Error>;
		}
	};
	let mut mock_client_builder = MockClient::new();
	mock_client_builder
		.expect_build()
		.once()
		.returning(move || Ok(mock_client.clone()));

	let result = generate_keys_via_rpc(&config, &mock_context).await;
	assert!(result.is_err());
	assert!(result.unwrap_err().to_string().contains("unsafe"));
}

#[tokio::test]
async fn test_parse_session_keys_with_invalid_hex() {
	let mock_context =
		MockIOContext::new().with_expected_io(vec![MockIO::eprint("🔍 Parsing session keys...")]);

	let keys_hex = "invalid_hex";
	let key_types = vec!["aura".to_string(), "gran".to_string()];
	let result = parse_session_keys_hex(&keys_hex, &key_types, &mock_context);
	assert!(result.is_err());
	assert!(result.unwrap_err().to_string().contains("Invalid hex string"));
}

#[tokio::test]
async fn test_parse_session_keys_with_short_hex() {
	let mock_context = MockIOContext::new().with_expected_io(vec![
		MockIO::eprint("🔍 Parsing session keys..."),
		MockIO::eprint("  ⚠️  Not enough data for key type: aura"),
		MockIO::eprint("  ⚠️  Not enough data for key type: gran"),
		MockIO::eprint("  ⚠️  Could not parse individual keys - providing full hex as raw"),
	]);

	let keys_hex = "0x1234567890abcdef";
	let key_types = vec!["aura".to_string(), "gran".to_string()];
	let session_keys = parse_session_keys_hex(&keys_hex, &key_types, &mock_context).unwrap();

	assert_eq!(session_keys.len(), 1);
	assert_eq!(session_keys[0].key_type, "raw");
	assert_eq!(session_keys[0].public_key, keys_hex);
}

#[test]
fn test_session_key_info_serialization() {
	let session_key = SessionKeyInfo {
		key_type: "aura".to_string(),
		public_key: "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef"
			.to_string(),
	};

	let json = serde_json::to_string(&session_key).unwrap();
	assert!(json.contains("aura"));
	assert!(json.contains("0x1234567890abcdef"));

	let deserialized: SessionKeyInfo = serde_json::from_str(&json).unwrap();
	assert_eq!(deserialized.key_type, "aura");
	assert_eq!(
		deserialized.public_key,
		"0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef"
	);
}

#[test]
fn test_hex_decoding() {
	// Test valid hex with 0x prefix
	let hex_with_prefix = "0x1234abcd";
	let stripped = hex_with_prefix.strip_prefix("0x").unwrap_or(hex_with_prefix);
	let decoded = hex::decode(stripped);
	assert!(decoded.is_ok());
	assert_eq!(decoded.unwrap(), vec![0x12, 0x34, 0xab, 0xcd]);

	// Test valid hex without prefix
	let hex_without_prefix = "1234abcd";
	let decoded = hex::decode(hex_without_prefix);
	assert!(decoded.is_ok());
	assert_eq!(decoded.unwrap(), vec![0x12, 0x34, 0xab, 0xcd]);

	// Test invalid hex
	let invalid_hex = "invalid_hex";
	let decoded = hex::decode(invalid_hex);
	assert!(decoded.is_err());
}
