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
        # Check if we should use kubectl exec and if required env vars are set
        use_kubectl = os.environ.get("USE_KUBECTL_RPC") == "true"
        kubectl_pod_exists = "KUBECTL_EXEC_POD" in os.environ
        
        # If kubectl mode is desired but required env var is missing, log warning and fall back
        if use_kubectl and not kubectl_pod_exists:
            logger.warning("USE_KUBECTL_RPC is set to true but KUBECTL_EXEC_POD is not defined. Falling back to HTTP request.")
            use_kubectl = False
        
        # Use the original HTTP method implementation
        if not use_kubectl:
            response = requests.post(
                self.url,
                headers=self.headers,
                json=self.__get_body(method=method, params=params)
            )
            return response.json()
        
        # Use kubectl exec method
        pod = os.environ["KUBECTL_EXEC_POD"]
        namespace = os.environ.get("K8S_NAMESPACE", "default")
        
        payload = json.dumps(self.__get_body(method=method, params=params))
        cmd = [
            "kubectl", "exec", pod, "-n", namespace, "--",
            "curl", "-s", "-H", "Content-Type: application/json",
            "-d", payload, self.url
        ]
        
        try:
            result = subprocess.run(cmd, capture_output=True, text=True, check=True)
            return json.loads(result.stdout)
        except subprocess.CalledProcessError as e:
            logger.error(f"kubectl exec command failed: {e.stderr}")
            raise PartnerChainRpcException(f"kubectl exec command failed: {e.stderr}")
        except json.JSONDecodeError as e:
            logger.error(f"Failed to parse JSON from kubectl response: {e}")
            raise PartnerChainRpcException(f"Failed to parse JSON from kubectl response: {e}")

    def partner_chain_get_epoch_committee(self, epoch) -> PartnerChainRpcResponse:
        json_data = self.__exec_rpc("sidechain_getEpochCommittee", [epoch])
        logger.debug(json_data)
        return PartnerChainRpcResponse.model_validate(json_data)

    def partner_chain_get_status(self):
        json_data = self.__exec_rpc("sidechain_getStatus")
        logger.debug(json_data)
        return PartnerChainRpcResponse.model_validate(json_data)

    def partner_chain_get_ariadne_parameters(self, mc_epoch) -> PartnerChainRpcResponse:
        json_data = self.__exec_rpc("sidechain_getAriadneParameters", [mc_epoch])
        logger.debug(json_data)
        return PartnerChainRpcResponse.model_validate(json_data)

    def partner_chain_get_params(self):
        json_data = self.__exec_rpc("sidechain_getParams")
        logger.debug(json_data)
        return PartnerChainRpcResponse.model_validate(json_data)

    def partner_chain_get_registrations(self, mc_epoch, mc_key):
        json_data = self.__exec_rpc("sidechain_getRegistrations", [mc_epoch, mc_key])
        logger.debug(json_data)
        return PartnerChainRpcResponse.model_validate(json_data)

    def partner_chain_get_block_producer_metadata(self, cross_chain_pub_key_hash: str):
        json_data = self.__exec_rpc("block-producer-metadata_getMetadata", [f"0x{cross_chain_pub_key_hash}"])
        logger.debug(json_data)
        return PartnerChainRpcResponse.model_validate(json_data)