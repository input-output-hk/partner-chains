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

    def __exec_rpc(self, method: str, params: list = []) -> PartnerChainRpcResponse:
        if os.environ.get("USE_KUBECTL_RPC") != "true":
            response = requests.post(
                self.url,
                headers=self.headers,
                json=self.__get_body(method=method, params=params)
            )
            json_data = response.json()
            logger.debug(json_data)
            return PartnerChainRpcResponse.model_validate(json_data)

        pod = os.environ["KUBECTL_EXEC_POD"]
        namespace = os.environ.get("K8S_NAMESPACE", "default")

        payload = json.dumps(self.__get_body(method=method, params=params))
        cmd = [
            "kubectl", "exec", pod, "-n", namespace, "--",
            "curl", "-s", "-H", "Content-Type: application/json",
            "-d", payload, "http://localhost:9933"
        ]
        result = subprocess.run(cmd, capture_output=True, text=True, check=True)
        json_data = json.loads(result.stdout)
        logger.debug(json_data)
        return PartnerChainRpcResponse.model_validate(json_data)

    def partner_chain_get_epoch_committee(self, epoch) -> PartnerChainRpcResponse:
        return self.__exec_rpc("sidechain_getEpochCommittee", [epoch])

    def partner_chain_get_status(self) -> PartnerChainRpcResponse:
        return self.__exec_rpc("sidechain_getStatus")

    def partner_chain_get_ariadne_parameters(self, mc_epoch) -> PartnerChainRpcResponse:
        return self.__exec_rpc("sidechain_getAriadneParameters", [mc_epoch])

    def partner_chain_get_params(self) -> PartnerChainRpcResponse:
        return self.__exec_rpc("sidechain_getParams")

    def partner_chain_get_registrations(self, mc_epoch, mc_key) -> PartnerChainRpcResponse:
        return self.__exec_rpc("sidechain_getRegistrations", [mc_epoch, mc_key])

    def partner_chain_get_block_producer_metadata(self, cross_chain_pub_key_hash: str) -> PartnerChainRpcResponse:
        return self.__exec_rpc("block-producer-metadata_getMetadata", [f"0x{cross_chain_pub_key_hash}"])
