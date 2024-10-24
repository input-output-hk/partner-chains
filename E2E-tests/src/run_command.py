import io
import logging
import paramiko
import subprocess
import yaml
from abc import ABC, abstractmethod
from config.api_config import SSH
import time


STDOUT_MAX_LEN = 2000


class Result:
    def __init__(self, stdout: str, stderr: str):
        self.stdout = stdout
        self.stderr = stderr

    def __repr__(self) -> str:
        return f"Result(stdout='{self.stdout}', stderr='{self.stderr}')"


class RunnerFactory:
    @staticmethod
    def get_runner(ssh: SSH, shell: str):
        if ssh:
            return SSHRunner(ssh)
        else:
            return LocalRunner(shell)


class Runner(ABC):
    @abstractmethod
    def run(self, command: str, timeout=120) -> Result:
        """Run any command.

        Currently supports two types of runners: LocalRunner and SSHRunner.

        LocalRunner is used when no SSH configuration is provided.
        It uses subprocess.run to execute the command with shell=True.

        SSHRunner is used when SSH configuration is provided.
        It uses paramiko.SSHClient to establish an SSH connection and execute the command.
        It uses get_pty=True to allocate a pseudo-terminal for the command (needed for saving logs to a file
        on remote host for some commands). This implies that the stderr is merged into stdout, see bug ->
        https://github.com/paramiko/paramiko/issues/1142

        Arguments:
            command {str} -- command to run

        Keyword Arguments:
            timeout {int} -- default: 120s

        Returns:
            Result -- object containing stdout and stderr
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
        if self.shell.split(" "):
            executable = None
            command = "{shell} \"{command}\"".format(shell=self.shell, command=command)

        try:
            completed_process = subprocess.run(
                command,
                timeout=timeout,
                capture_output=True,
                shell=True,
                executable=executable,
                encoding="utf-8",
            )
            result = Result(stdout=completed_process.stdout, stderr=completed_process.stderr)
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


class SSHRunner(Runner):
    def __init__(self, ssh_config: SSH):
        self.host = ssh_config.host
        self.port = ssh_config.port
        self.user = ssh_config.username
        self.key_path = ssh_config.private_key_path
        self.client = paramiko.SSHClient()
        if ssh_config.host_keys_path:
            self.client.load_host_keys(ssh_config.host_keys_path)

    def load_key_from_yaml(self, path):
        with open(path, "r") as file:
            key = yaml.safe_load(file)["ssh_key"]
            return key

    def connect(self):
        logging.debug(f"SSH: Connecting to {self.host}:{self.port} as {self.user}")
        try:
            private_key = paramiko.RSAKey.from_private_key(io.StringIO(self.load_key_from_yaml(self.key_path)))
            self.client.connect(self.host, self.port, self.user, pkey=private_key)
        except paramiko.AuthenticationException as auth_err:
            logging.error(f"Authentication failed: {auth_err}")
            raise auth_err
        except paramiko.SSHException as ssh_err:
            logging.error(f"Unable to establish SSH connection: {ssh_err}")
            raise ssh_err
        except Exception as e:
            logging.error(f"An error occurred: {e}")
            raise e

        return None

    def run(self, command: str, timeout=120) -> Result:
        self.connect()
        logging.debug(f"CMD: '{command}' TIMEOUT: {timeout}")
        try:
            _, stdout, stderr = self.client.exec_command(command, get_pty=True, timeout=timeout)

            # Wait until we can read the channel.
            # We check stdout only because stderr is merged into in with get_pty=True
            end_time = time.time() + timeout
            while not stdout.channel.exit_status_ready():
                time.sleep(1)
                if time.time() > end_time:
                    raise TimeoutError(f"Command '{command}' timed out after {timeout}s")
            output = stdout.read().decode()

            # this blocks execution until the command finishes, but it should be merged already into stdout
            stderr.channel.recv_exit_status()
            error = stderr.read().decode()

            result = Result(stdout=output, stderr=error)
            truncated_output = (
                result.stdout[:STDOUT_MAX_LEN] + "..." if len(result.stdout) > STDOUT_MAX_LEN else result.stdout
            )
            logging.debug(f"STDOUT: {truncated_output}")
            if result.stderr:
                logging.error(f"STDERR: {result.stderr}")
            return result
        except TimeoutError as e:
            logging.error(f"TIMEOUT: {e}")
            raise e
        except Exception as e:
            logging.error(f"UNKNOWN ERROR: {e}")
            raise e
        finally:
            self.close()

    def close(self):
        self.client.close()
