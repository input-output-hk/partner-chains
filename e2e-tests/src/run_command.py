import logging
import subprocess
from abc import ABC, abstractmethod
from config.api_config import RunnerConfig

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
            return KubernetesRunner(cfg)
        elif cfg.docker:
            return DockerRunner(cfg)
        else:
            raise ValueError(
                "No valid runner configuration provided. Please specify either Kubernetes or Docker configuration."
            )


class Runner(ABC):
    workdir: str
    copy_secrets: bool

    @abstractmethod
    def exec(self, command: str, timeout=120) -> Result:
        """Execute a command in the runner environment."""
        raise NotImplementedError("exec method must be implemented in subclasses")

    @abstractmethod
    def copy(self, src: str, dest: str) -> Result:
        """Copy a file from local to remote."""
        raise NotImplementedError("copy method must be implemented in subclasses")

    @abstractmethod
    def mktemp(self) -> str:
        """Create a temporary file in the runner environment."""
        raise NotImplementedError("mktemp method must be implemented in subclasses")

    @abstractmethod
    def cleanup(self) -> None:
        """Cleanup any resources or temporary files created by the runner."""
        raise NotImplementedError("cleanup method must be implemented in subclasses")


class KubernetesRunner(Runner):
    def __init__(self, config: RunnerConfig):
        self.pod = config.kubernetes.pod
        self.namespace = config.kubernetes.namespace
        self.container = config.kubernetes.container
        self.copy_secrets = config.copy_secrets
        self.workdir = config.workdir
        self.workdir_created = False
        self.files_created = []
        if self.workdir:
            self.create_working_directory()

    def _run(self, cmd: str, timeout=120, suppress_stderr_logs=False) -> Result:
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
            if result.stderr and not suppress_stderr_logs:
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
            command = f"cd {self.workdir} && {command}"
        cmd = f"kubectl exec {self.pod} -c {self.container} -n {self.namespace} -- bash -c \"{command}\""
        return self._run(cmd, timeout)

    def copy(self, src: str, dest: str) -> Result:
        cmd = f"kubectl cp {src} {self.pod}:{dest} -c {self.container} -n {self.namespace}"
        return self._run(cmd)

    def mktemp(self) -> str:
        command = "mktemp"
        if self.workdir:
            command = f"{command} -p {self.workdir}"
        full_command = f"kubectl exec {self.pod} -c {self.container} -n {self.namespace} -- bash -c \"{command}\""
        result = self._run(full_command)
        if result.returncode != 0:
            raise RuntimeError(f"Failed to create temporary directory: {result.stderr}")
        temp_file = result.stdout.strip()
        self.files_created.append(temp_file)
        logging.debug(f"Temporary file created: {temp_file}")
        return temp_file

    def cleanup(self) -> None:
        if not self.files_created:
            logging.info("No temporary files to remove.")
            return
        logging.info(f"Removing temporary files: {self.files_created}")
        cmd = f"rm {' '.join(self.files_created)}"
        full_cmd = f"kubectl exec {self.pod} -c {self.container} -n {self.namespace} -- {cmd}"
        self._run(full_cmd)

    def create_working_directory(self) -> str:
        if not self.workdir or self.workdir_created:
            return
        exists_cmd = f"kubectl exec {self.pod} -c {self.container} -n {self.namespace} -- test -d {self.workdir}"
        result = self._run(exists_cmd, suppress_stderr_logs=True)
        if result.returncode == 0:
            return
        logging.info(f"Creating working directory {self.workdir} in kubernetes container {self.container}")
        create_cmd = f"kubectl exec {self.pod} -c {self.container} -n {self.namespace} -- mkdir -p {self.workdir}"
        result = self._run(create_cmd)
        if result.returncode != 0:
            raise RuntimeError(f"Failed to create working directory: {result.stderr}")
        self.workdir_created = True


class DockerRunner(Runner):
    pass
