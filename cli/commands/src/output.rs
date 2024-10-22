pub type OutputLine = Vec<String>;

pub fn payment_signing_key_line_without_value() -> OutputLine {
	vec!["--payment-signing-key-file".to_string(), "<PATH_TO_SIGNING_KEY_FILE>".to_string()]
}

pub fn pretty_output(lines: Vec<OutputLine>) -> String {
	lines
		.into_iter()
		.filter(|v| !v.is_empty())
		.map(|line| line.into_iter().map(|item| item + " ").collect::<String>() + "\\\n")
		.collect()
}
