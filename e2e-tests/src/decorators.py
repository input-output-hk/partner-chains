import signal


def timeout_handler(signum, frame):
    raise TimeoutError(f"Function execution timed out {signum} {frame}")


def long_running_function(func):
    def wrapper(*args, **kwargs):
        signal.signal(signal.SIGALRM, timeout_handler)
        signal.alarm(args[0].config.timeouts.long_running_function)
        try:
            return func(*args, **kwargs)
        finally:
            signal.alarm(0)

    return wrapper
