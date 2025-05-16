import io
import logging
import paramiko
import subprocess
import yaml
import time
import shlex
from typing import Optional
from scp import SCPClient
from abc import ABC, abstractmethod
from config.api_config import SSH

STDOUT_MAX_LEN = 2000


class Result:
    def __init__(self, returncode: int, stdout: str, stderr: str):
        self.returncode = returncode
        self.stdout = stdout
        self.stderr = stderr

    def __repr__(self) -> str:
        return f"Result(stdout='{self.stdout}', stderr='{self.stderr}')"


class Runner(ABC):
    @abstractmethod
    def run(self, command: str, timeout=120) -> Result:
        """Run any command.

        Currently supports LocalRunner, SSHRunner, and KubectlRunner.
        It uses subprocess.run to execute the command with shell=True.

        Arguments:
            command {str} -- command to run

        Keyword Arguments:
            timeout {int} -- default: 120s

        Returns:
            Result -- object containing returncode, stdout and stderr
        """
        pass


class RunnerFactory:
    @staticmethod
    def get_runner(ssh: Optional[SSH], shell: str, kubectl: Optional[dict] = None):
        """
        - if kubectl is provided, return KubectlRunner(pod, namespace, container)
        - elif ssh is provided, return SSHRunner
        - else fall back to LocalRunner
        """
        if kubectl:
            return KubectlRunner(
                pod=kubectl["pod"],
                namespace=kubectl.get("namespace", "default"),
                container=kubectl.get("container"),
            )
        if ssh:
            return SSHRunner(ssh)
        return LocalRunner(shell)


class LocalRunner(Runner):
    def __init__(self, shell: str = None):
        self.shell = shell

    def run(self, command: str, timeout=120) -> Result:
        logging.debug(f"CMD: '{command}' TIMEOUT: {timeout} SHELL: {self.shell}")

        executable = self.shell
        if self.shell and self.shell.split(" "):
            # wrap in quotes to preserve spaces
            escaped = command.replace('"', '\\"')
            command = f"{self.shell} \"{escaped}\""
            executable = None

        try:
            cp = subprocess.run(
                command,
                timeout=timeout,
                capture_output=True,
                shell=True,
                executable=executable,
                encoding="utf-8",
            )
            result = Result(cp.returncode, cp.stdout, cp.stderr)
            out = (result.stdout[:STDOUT_MAX_LEN] + "...") if len(result.stdout) > STDOUT_MAX_LEN else result.stdout
            logging.debug(f"STDOUT: {out}")
            if result.stderr:
                logging.warning(f"STDERR: {result.stderr}")
            return result
        except subprocess.TimeoutExpired as e:
            logging.error(f"TIMEOUT: {e}")
            raise
        except Exception as e:
            logging.error(f"UNKNOWN ERROR: {e}")
            raise

    def _cmd(self, cli, cmd) -> str:
        full = f"{cli} {cmd}"
        if self.shell:
            full = f"{self.shell} \"{cli} {cmd}\""
        return full


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
        with open(path, "r") as f:
            return yaml.safe_load(f)["ssh_key"]

    def connect(self):
        logging.debug(f"SSH CONNECT → {self.user}@{self.host}:{self.port}")
        key_str = self.load_key_from_yaml(self.key_path)
        key = paramiko.RSAKey.from_private_key(io.StringIO(key_str))
        self.client.connect(self.host, self.port, self.user, pkey=key)

    def run(self, command: str, timeout=120) -> Result:
        self.connect()
        logging.debug(f"SSH CMD '{command}' TIMEOUT {timeout}")
        try:
            _, stdout, stderr = self.client.exec_command(command, timeout=timeout)
            end = time.time() + timeout
            while not stdout.channel.exit_status_ready():
                if time.time() > end:
                    raise TimeoutError(f"'{command}' timed out")
                time.sleep(1)
            out = stdout.read().decode()
            code = stdout.channel.recv_exit_status()
            err = stderr.read().decode()
            result = Result(code, out, err)
            short = (out[:STDOUT_MAX_LEN] + "...") if len(out) > STDOUT_MAX_LEN else out
            logging.debug(f"SSH STDOUT: {short}")
            if err:
                logging.warning(f"SSH STDERR: {err}")
            return result
        finally:
            self.close()

    def scp(self, src, dst):
        self.connect()
        with SCPClient(self.client.get_transport()) as scp:
            scp.put(src, remote_path=dst)
        self.close()

    def close(self):
        self.client.close()
        logging.debug("SSH DISCONNECTED")


class KubectlRunner(Runner):
    def __init__(self, pod: str, namespace: str, container: Optional[str] = None):
        self.pod = pod
        self.ns = namespace
        self.container = container

    def run(self, command: str, timeout=120) -> Result:
        # build: kubectl exec <pod> -n <ns> [-c <container>] -- <command>
        cmd = ["kubectl", "exec", self.pod, "-n", self.ns]
        if self.container:
            cmd += ["-c", self.container]
        cmd += ["--"] + shlex.split(command)

        proc = subprocess.run(
            cmd, timeout=timeout, capture_output=True, text=True
        )
        return Result(proc.returncode, proc.stdout, proc.stderr)
