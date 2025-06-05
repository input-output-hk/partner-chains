import logging
import subprocess
from abc import ABC
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

    def __init__(self, config: RunnerConfig):
        self.copy_secrets = config.copy_secrets
        self.workdir = config.workdir
        self.workdir_created = False
        self.files_created = []
        if self.workdir:
            self.create_working_directory()

    def exec(self, command: str, timeout=120) -> Result:
        if self.workdir:
            command = f"cd {self.workdir} && {command}"
        return self._run(command, timeout)

    def mktemp(self) -> str:
        command = "mktemp"
        if self.workdir:
            command = f"{command} -p {self.workdir}"
        result = self._run(command)
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
        self._run(cmd)

    def create_working_directory(self) -> str:
        if not self.workdir or self.workdir_created:
            return

        result = self._run(f"test -d {self.workdir}", suppress_stderr_logs=True)
        if result.returncode == 0:
            self.workdir_created = True
            return

        logging.info(f"Creating working directory {self.workdir} in container {self.container}")
        result = self._run(f"mkdir -p {self.workdir}")
        if result.returncode != 0:
            raise RuntimeError(f"Failed to create working directory: {result.stderr}")
        self.workdir_created = True

    def _run(self, cmd: str, timeout=120, suppress_stderr_logs=False) -> Result:
        full_cmd = self._full_cmd(cmd)
        logging.debug(f"CMD: '{full_cmd}' TIMEOUT: {timeout}")
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
            if result.stderr and not suppress_stderr_logs:
                logging.warning(f"STDERR: {result.stderr}")
            return result
        except subprocess.TimeoutExpired as e:
            logging.error(f"TIMEOUT: {e}")
            raise e
        except Exception as e:
            logging.error(f"UNKNOWN ERROR: {e}")
            raise e


class KubernetesRunner(Runner):
    def __init__(self, config: RunnerConfig):
        self.pod = config.kubernetes.pod
        self.namespace = config.kubernetes.namespace
        self.container = config.kubernetes.container
        super().__init__(config)

    def _full_cmd(self, command: str) -> str:
        return f"kubectl exec {self.pod} -c {self.container} -n {self.namespace} -- bash -c \"{command}\""


class DockerRunner(Runner):
    def __init__(self, config: RunnerConfig):
        self.container = config.docker.container
        super().__init__(config)

    def _full_cmd(self, command: str) -> str:
        return f"docker exec {self.container} bash -c \"{command}\""
