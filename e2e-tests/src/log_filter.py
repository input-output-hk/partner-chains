import logging
import re


class SensitiveDataFilter(logging.Filter):
    def __init__(self, patterns):
        super().__init__()
        self.patterns = patterns

    def filter(self, record):
        message = record.getMessage()
        for pattern, replacement in self.patterns:
            message = re.sub(pattern, replacement, message)
        record.msg = message
        return True


signing_key_arg_pattern = (
    re.compile(r"(--signing-key\s+|--mainchain-signing-key\s+|--sidechain-signing-key\s+)[^\s]+", re.IGNORECASE),
    r"\1[REDACTED]",
)
signing_key_file_pattern = (
    re.compile(r'(SigningKey.*?cborHex\\?"?:\s*\\?")([0-9a-fA-F]+)(.)', re.IGNORECASE | re.DOTALL),
    r"\1[REDACTED]\3",
)

sensitive_filter = SensitiveDataFilter([signing_key_arg_pattern, signing_key_file_pattern])
