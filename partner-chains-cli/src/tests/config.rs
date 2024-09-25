use crate::config::*;

mod config_field {

	use crate::tests::{MockIO, MockIOContext};

	use super::*;

	#[test]
	fn saves_to_new_file() {
		let config_file_path = "/path/to/test-config.json";

		let config_field: ConfigFieldDefinition<String> = ConfigFieldDefinition {
			name: "test config field",
			config_file: config_file_path,
			path: &["path", "to", "field"],
			default: None,
			_marker: Default::default(),
		};

		let expected_file_content = serde_json::json!({
			"path": {
				"to": {
					"field": "this is a test string"
				}
			}
		});

		let mock_context = MockIOContext::new().with_expected_io(vec![MockIO::file_write_json(
			config_file_path,
			expected_file_content,
		)]);

		config_field.save_to_file(&"this is a test string".into(), &mock_context);
	}

	#[test]
	fn saves_to_existing_file() {
		let config_file_path = "/path/to/test-config.json";

		let existing_content = serde_json::json!({
			"some": {
				"other": {
					"path": "some other string"
				}
			}
		});

		let config_field: ConfigFieldDefinition<String> = ConfigFieldDefinition {
			name: "test config field",
			config_file: config_file_path,
			path: &["path", "to", "field"],
			default: None,
			_marker: Default::default(),
		};

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

		let mock_context = MockIOContext::new()
			.with_json_file(config_file_path, existing_content)
			.with_expected_io(vec![
				MockIO::file_read(config_file_path),
				MockIO::file_write_json(config_file_path, expected_file_content),
			]);

		config_field.save_to_file(&"this is a test string".into(), &mock_context);
	}

	#[test]
	fn loads_file() {
		let config_file_path = "/path/to/test-config.json";

		let json_content = serde_json::json!({
			"path": {
				"to": {
					"field": "this is a test string"
				}
			}
		});

		let config_field: ConfigFieldDefinition<String> = ConfigFieldDefinition {
			name: "test config field",
			config_file: config_file_path,
			path: &["path", "to", "field"],
			default: None,
			_marker: Default::default(),
		};

		let mock_context = MockIOContext::new()
			.with_json_file(config_file_path, json_content.clone())
			.with_expected_io(vec![MockIO::file_read(config_file_path)]);

		let read_content = config_field.load_file(&mock_context);

		assert_eq!(read_content, Some(json_content));
	}

	#[test]
	fn extracts_from_json() {
		let config_field: ConfigFieldDefinition<String> = ConfigFieldDefinition {
			name: "test config field",
			config_file: "not used",
			path: &["path", "to", "field"],
			default: None,
			_marker: Default::default(),
		};

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
