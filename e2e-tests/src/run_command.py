import logging
import subprocess
from abc import ABC, abstractmethod
from typing import Optional


STDOUT_MAX_LEN = 2000


class Result:
    def __init__(self, returncode: int, stdout: str, stderr: str):
        self.returncode = returncode
        self.stdout = stdout
        self.stderr = stderr

    def __repr__(self) -> str:
        return f"Result(stdout='{self.stdout}', stderr='{self.stderr}')"


class RunnerFactory:
    @staticmethod
    def get_runner(ssh: Optional[None], shell: str):
        return LocalRunner(shell)


class Runner(ABC):
    @abstractmethod
    def run(self, command: str, timeout=120) -> Result:
        """Run any command.

        Currently only supports LocalRunner.
        It uses subprocess.run to execute the command with shell=True.

        Arguments:
            command {str} -- command to run

        Keyword Arguments:
            timeout {int} -- default: 120s

        Returns:
            Result -- object containing returncode, stdout and stderr
        """
        pass


class LocalRunner(Runner):
    def __init__(self, shell: str = None):
        self.shell = shell

    def _cmd(self, cli, cmd) -> str:
        full_cmd = "{cli} {cmd}".format(cli=cli, cmd=cmd)
        if self.shell:
            full_cmd = "{shell} \"{cli} {cmd}\"".format(shell=self.shell, cli=cli, cmd=cmd)
        return full_cmd

    def run(self, command: str, timeout: Optional[int] = None) -> Result:
        """Run a command and return the result."""
        try:
            completed_process = subprocess.run(
                command,
                timeout=timeout,
                capture_output=True,
                shell=True,
                executable=self.shell,
                encoding="utf-8",
            )
            return Result(
                returncode=completed_process.returncode,
                stdout=completed_process.stdout,
                stderr=completed_process.stderr,
            )
        except subprocess.TimeoutExpired:
            logging.error(f"Command timed out after {timeout} seconds")
            return Result(returncode=-1, stdout="", stderr=f"Command timed out after {timeout} seconds")
        except Exception as e:
            logging.error(f"Failed to run command: {e}")
            return Result(returncode=-1, stdout="", stderr=str(e))
