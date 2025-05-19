import requests
import logging as logger
from dataclasses import dataclass
from pydantic import BaseModel
from typing import Optional
import os
import subprocess
import json


class PartnerChainRpcResponseError(BaseModel):
    code: int
    message: str


class PartnerChainRpcResponse(BaseModel):
    id: int
    jsonrpc: str
    result: Optional[dict | list] = None
    error: Optional[PartnerChainRpcResponseError] = None


class PartnerChainRpcException(Exception):
    def __init__(self, message="PartnerChain RPC error occurred", status_code=None):
        self.message = message
        self.status_code = status_code
        super().__init__(self.message)


@dataclass
class DParam:
    permissioned_candidates_number: int
    trustless_candidates_number: int


class PartnerChainRpc:
    def __init__(self, url):
        self.url = url
        self.headers = {"Content-Type": "application/json; charset=utf-8"}

    def __get_body(self, jsonrpc="2.0", method="", params=[], id=1):
        return {
            "jsonrpc": jsonrpc,
            "method": method,
            "params": params,
            "id": id,
        }

    def __exec_rpc(self, method: str, params: list = []):
        """
        Execute an RPC request either via HTTP or kubectl exec.
        Returns the raw JSON response for further processing.
        """
        try:
            if os.environ.get("USE_KUBECTL_RPC") != "true":
                # Use standard HTTP request method
                response = requests.post(
                    self.url, headers=self.headers, json=self.__get_body(method=method, params=params)
                )
                return response.json()
            else:
                # Use kubectl exec method
                pod = os.environ["KUBECTL_EXEC_POD"]
                namespace = os.environ.get("K8S_NAMESPACE", "default")

                payload = json.dumps(self.__get_body(method=method, params=params))
                cmd = [
                    "kubectl",
                    "exec",
                    pod,
                    "-n",
                    namespace,
                    "--",
                    "curl",
                    "-s",
                    "-H",
                    "Content-Type: application/json",
                    "-d",
                    payload,
                    "http://localhost:9933",
                ]
                result = subprocess.run(cmd, capture_output=True, text=True, check=True)
                if not result.stdout.strip():
                    raise ValueError(f"Empty response from kubectl exec command: {cmd}")
                return json.loads(result.stdout)
        except subprocess.CalledProcessError as e:
            logger.error(f"Command failed: {e.cmd}, exit_code: {e.returncode}")
            logger.error(f"stderr: {e.stderr}")
            raise PartnerChainRpcException(f"Command execution failed: {e.stderr}", e.returncode)
        except json.JSONDecodeError as e:
            logger.error(f"Failed to parse JSON response: {e}")
            raise PartnerChainRpcException(f"Invalid JSON response: {e}")
        except Exception as e:
            logger.error(f"RPC execution error: {str(e)}")
            raise PartnerChainRpcException(str(e))

    def partner_chain_get_epoch_committee(self, epoch) -> PartnerChainRpcResponse:
        json_data = self.__exec_rpc("sidechain_getEpochCommittee", [epoch])
        logger.debug(json_data)
        return PartnerChainRpcResponse.model_validate(json_data)

    def partner_chain_get_status(self) -> PartnerChainRpcResponse:
        json_data = self.__exec_rpc("sidechain_getStatus")
        logger.debug(json_data)
        return PartnerChainRpcResponse.model_validate(json_data)

    def partner_chain_get_ariadne_parameters(self, mc_epoch) -> PartnerChainRpcResponse:
        json_data = self.__exec_rpc("sidechain_getAriadneParameters", [mc_epoch])
        logger.debug(json_data)
        return PartnerChainRpcResponse.model_validate(json_data)

    def partner_chain_get_params(self) -> PartnerChainRpcResponse:
        json_data = self.__exec_rpc("sidechain_getParams")
        logger.debug(json_data)
        return PartnerChainRpcResponse.model_validate(json_data)

    def partner_chain_get_registrations(self, mc_epoch, mc_key) -> PartnerChainRpcResponse:
        json_data = self.__exec_rpc("sidechain_getRegistrations", [mc_epoch, mc_key])
        logger.debug(json_data)
        return PartnerChainRpcResponse.model_validate(json_data)

    def partner_chain_get_block_producer_metadata(self, cross_chain_pub_key_hash: str) -> PartnerChainRpcResponse:
        json_data = self.__exec_rpc("pc_getMetadata", [f"0x{cross_chain_pub_key_hash}"])
        logger.debug(json_data)
        return PartnerChainRpcResponse.model_validate(json_data)

    def partner_chain_get_block_producer_fees(self) -> PartnerChainRpcResponse:
        json_data = self.__exec_rpc("pc_getBlockProducerFees")
        logger.debug(json_data)
        return PartnerChainRpcResponse.model_validate(json_data)
