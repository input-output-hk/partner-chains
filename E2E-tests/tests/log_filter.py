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


# Create the filter with a pattern to match mc_vkey
sensitive_filter = SensitiveDataFilter([(r"mc_vkey='([^']*)'", "mc_vkey='[REDACTED]'")])
