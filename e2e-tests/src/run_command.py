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

    def run(self, command: str, timeout=120) -> Result:
        logging.debug(f"CMD: '{command}' TIMEOUT: {timeout} SHELL: {self.shell}")

        executable = self.shell
        if self.shell and self.shell.split(" "):
            executable = None
            escaped_command = command.replace('"', '\\"')
            command = "{shell} \"{command}\"".format(shell=self.shell, command=escaped_command)

        try:
            completed_process = subprocess.run(
                command,
                timeout=timeout,
                capture_output=True,
                shell=True,
                executable=executable,
                encoding="utf-8",
            )
            result = Result(
                returncode=completed_process.returncode,
                stdout=completed_process.stdout,
                stderr=completed_process.stderr,
            )
            truncated_output = (
                result.stdout[:STDOUT_MAX_LEN] + "..." if len(result.stdout) > STDOUT_MAX_LEN else result.stdout
            )
            logging.debug(f"STDOUT: {truncated_output}")
            if result.stderr:
                logging.error(f"STDERR: {result.stderr}")
            return result
        except subprocess.TimeoutExpired as e:
            logging.error(f"TIMEOUT: {e}")
            raise e
        except Exception as e:
            logging.error(f"UNKNOWN ERROR: {e}")
            raise e
