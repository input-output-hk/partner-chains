import logging
import subprocess
from abc import ABC, abstractmethod
from config.api_config import RunnerConfig, DockerConfig, KubernetesConfig, FilesConfig

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
    def get_runner(cfg: RunnerConfig) -> 'Runner':
        if cfg.kubernetes:
            return KubernetesRunner(cfg.kubernetes, cfg.files)
        elif cfg.docker:
            return DockerRunner(cfg.docker, cfg.files)
        else:
            raise ValueError(
                "No valid runner configuration provided. Please specify either Kubernetes or Docker configuration."
            )


class Runner(ABC):
    files_config: FilesConfig

    @abstractmethod
    def exec(self, command: str, timeout=120) -> Result:
        """Execute a command in the runner environment."""
        raise NotImplementedError("exec method must be implemented in subclasses")

    @abstractmethod
    def copy(self, src: str, dest: str) -> Result:
        """Copy a file from local to remote."""
        raise NotImplementedError("copy method must be implemented in subclasses")


class KubernetesRunner(Runner):
    def __init__(self, config: KubernetesConfig, files_config: FilesConfig):
        self.files_config = files_config
        self.pod = config.pod
        self.namespace = config.namespace
        self.container = config.container
        self.workdir = config.workdir

    def _run(self, cmd: str, timeout=120) -> str:
        logging.debug(f"CMD: '{cmd}' TIMEOUT: {timeout}")
        try:
            completed_process = subprocess.run(
                cmd,
                timeout=timeout,
                capture_output=True,
                shell=True,
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
                logging.warning(f"STDERR: {result.stderr}")
            return result
        except subprocess.TimeoutExpired as e:
            logging.error(f"TIMEOUT: {e}")
            raise e
        except Exception as e:
            logging.error(f"UNKNOWN ERROR: {e}")
            raise e

    def exec(self, command: str, timeout=120) -> Result:
        if self.workdir:
            command = (
                f"mkdir -p {self.workdir} && cd {self.workdir} && mkdir -p {self.files_config.copy_to} && {command}"
            )
        cmd = f"kubectl exec {self.pod} -c {self.container} -n {self.namespace} -- bash -c \"{command}\""
        return self._run(cmd, timeout)

    def copy(self, src: str, dest: str) -> Result:
        cmd = f"kubectl cp {src} {self.pod}:{dest} -c {self.container} -n {self.namespace}"
        return self._run(cmd)


class DockerRunner(Runner):
    def __init__(self, config: DockerConfig, files_config: FilesConfig):
        self.files_config = files_config
        self.container = config.container

    def _cmd(self, cli, cmd) -> str:
        return f"docker exec {self.container} {cli} {cmd}"

    def exec(self, command: str, timeout=120) -> Result:
        logging.debug(f"CMD: '{command}' TIMEOUT: {timeout} CONTAINER: {self.container}")

        full_cmd = self._cmd("bash", command)
        try:
            completed_process = subprocess.run(
                full_cmd,
                timeout=timeout,
                capture_output=True,
                shell=True,
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
                logging.warning(f"STDERR: {result.stderr}")
            return result
        except subprocess.TimeoutExpired as e:
            logging.error(f"TIMEOUT: {e}")
            raise e
        except Exception as e:
            logging.error(f"UNKNOWN ERROR: {e}")
            raise e
