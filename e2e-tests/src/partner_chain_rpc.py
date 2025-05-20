import requests
import logging as logger
from dataclasses import dataclass
from pydantic import BaseModel
from typing import Optional, Any, Dict
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

    def __exec_rpc(self, method: str, params: Optional[list] = None) -> Dict[str, Any]:
        """
        Execute an RPC request to the partner chain node via HTTP or kubectl exec.
        """
        if params is None:
            params = []

        body = self.__get_body(method=method, params=params)
        try:
            if os.environ.get("USE_KUBECTL_RPC") != "true":
                # standard HTTP call
                response = requests.post(
                    self.url,
                    headers=self.headers,
                    json=body
                )
                return response.json()
            else:
                # kubectl exec
                pod = os.environ["KUBECTL_EXEC_POD"]
                namespace = os.environ.get("K8S_NAMESPACE", "default")
                payload = json.dumps(body)
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
                return json.loads(result.stdout)
        except Exception as e:
            logger.error(f"RPC execution error: {e}")
            raise PartnerChainRpcException(str(e))

    def partner_chain_get_epoch_committee(self, epoch) -> PartnerChainRpcResponse:
        json_data = self.__exec_rpc("sidechain_getEpochCommittee", [epoch])
        logger.debug(json_data)
        return PartnerChainRpcResponse.model_validate(json_data)

    def partner_chain_get_status(self):
        json_data = self.__exec_rpc("sidechain_getStatus")
        logger.debug(json_data)
        return PartnerChainRpcResponse.model_validate(json_data)

    def partner_chain_get_ariadne_parameters(self, mc_epoch) -> PartnerChainRpcResponse:
        try:
            logger.info(f"Fetching Ariadne parameters for MC epoch: {mc_epoch}")
            json_data = self.__exec_rpc("sidechain_getAriadneParameters", [mc_epoch])
            logger.debug(f"Got Ariadne parameters response: {json.dumps(json_data, indent=2)}")
            
            if not json_data:
                logger.error("Received empty response from RPC call")
                return PartnerChainRpcResponse(error={"message": "Empty response received"}, result=None)
                
            if "result" not in json_data:
                logger.error(f"Invalid response format - missing 'result' field. Full response: {json.dumps(json_data, indent=2)}")
                return PartnerChainRpcResponse(error={"message": "Invalid response format - missing result"}, result=None)
                
            if "error" in json_data and json_data["error"]:
                logger.error(f"RPC call returned error in response: {json.dumps(json_data['error'], indent=2)}")
                return PartnerChainRpcResponse(error=json_data["error"], result=None)
                
            # Log specific parameter counts if available
            if "result" in json_data and isinstance(json_data["result"], dict):
                if "permissionedCandidates" in json_data["result"]:
                    pc = json_data["result"]["permissionedCandidates"]
                    logger.info(f"Found {len(pc) if pc is not None else 'None'} permissioned candidates")
                if "candidateRegistrations" in json_data["result"]:
                    cr = json_data["result"]["candidateRegistrations"]
                    logger.info(f"Found {len(cr) if cr is not None else 'None'} candidate registrations")
                if "dParameter" in json_data["result"]:
                    dp = json_data["result"]["dParameter"]
                    logger.info(f"D-Parameter values: {json.dumps(dp, indent=2)}")
                
            return PartnerChainRpcResponse.model_validate(json_data)
            
        except Exception as e:
            logger.error(f"Unexpected error getting Ariadne parameters: {e}")
            logger.exception("Full exception details:")
            return PartnerChainRpcResponse(error={"message": str(e)}, result=None)

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

    def partner_chain_get_block_producer_fees(self) -> PartnerChainRpcResponse:
        json_data = self.__exec_rpc("pc_getBlockProducerFees")
        logger.debug(json_data)
        return PartnerChainRpcResponse.model_validate(json_data)
