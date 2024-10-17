import re
import logging


class PasswordFilter(logging.Filter):
    """Filter class to obscure sensitive information from logging."""

    def __init__(self, pattern):
        self.pattern = pattern

    def filter(self, record):
        record.msg = re.sub(self.pattern, r"\1********", str(record.msg))
        return True
