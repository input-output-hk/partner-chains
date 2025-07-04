use crate::config::*;

mod config_field {

	use crate::{
		tests::{CHAIN_CONFIG_FILE_PATH, MockIOContext, RESOURCES_CONFIG_FILE_PATH},
		verify_json,
	};

	use super::*;

	#[test]
	fn saves_to_new_file() {
		let config_field: ConfigFieldDefinition<String> = ConfigFieldDefinition::new(
			"test config field",
			ConfigFile::Chain,
			&["path", "to", "field"],
			None,
		);

		let expected_file_content = serde_json::json!({
			"path": {
				"to": {
					"field": "this is a test string"
				}
			}
		});

		let mock_context = MockIOContext::new();

		config_field.save_to_file(&"this is a test string".into(), &mock_context);
		verify_json!(mock_context, CHAIN_CONFIG_FILE_PATH, expected_file_content);
	}

	#[test]
	fn saves_to_existing_file() {
		let existing_content = serde_json::json!({
			"some": {
				"other": {
					"path": "some other string"
				}
			}
		});

		let config_field: ConfigFieldDefinition<String> = ConfigFieldDefinition::new(
			"test config field",
			ConfigFile::Resources,
			&["path", "to", "field"],
			None,
		);

		let expected_file_content = serde_json::json!({
			"path": {
				"to": {
					"field": "this is a test string"
				}
			},
			"some": {
				"other": {
					"path": "some other string"
				}
			}
		});

		let mock_context =
			MockIOContext::new().with_json_file(RESOURCES_CONFIG_FILE_PATH, existing_content);

		config_field.save_to_file(&"this is a test string".into(), &mock_context);
		verify_json!(mock_context, RESOURCES_CONFIG_FILE_PATH, expected_file_content);
	}

	#[test]
	fn loads_file() {
		let json_content = serde_json::json!({
			"path": {
				"to": {
					"field": "this is a test string"
				}
			}
		});

		let config_field: ConfigFieldDefinition<String> = ConfigFieldDefinition::new(
			"test config field",
			ConfigFile::Resources,
			&["path", "to", "field"],
			None,
		);

		let mock_context =
			MockIOContext::new().with_json_file(RESOURCES_CONFIG_FILE_PATH, json_content.clone());

		let read_content = config_field.load_file(&mock_context);

		assert_eq!(read_content, Some(json_content));
	}

	#[test]
	fn extracts_from_json() {
		let config_field: ConfigFieldDefinition<String> = ConfigFieldDefinition::new(
			"test config field",
			ConfigFile::Chain,
			&["path", "to", "field"],
			None,
		);

		let json_object = serde_json::json!({
			"path": {
				"to": {
					"field": "this is the expected string",
					"another field": "in this object"
				}
			},
			"some other field": 9,
			"some": {
				"other": {
					"path": "unexpected string"
				}
			}
		});

		let extracted_value = config_field.extract_from_json_object(&json_object);

		assert_eq!(extracted_value, Some(String::from("this is the expected string")))
	}
}
