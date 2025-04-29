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
        Execute an RPC call to the partner chain node.

        :param method: The RPC method to call
        :param params: Optional parameters for the RPC call
        :return: The JSON response from the RPC call
        """
        if params is None:
            params = []

        data = {
            "jsonrpc": "2.0",
            "method": method,
            "params": params,
            "id": 1,
        }

        try:
            logger.info(f"Making RPC call to {self.url}")
            logger.info(f"Method: {method}")
            logger.info(f"Parameters: {params}")
            logger.debug(f"Full request data: {json.dumps(data, indent=2)}")
            
            result = subprocess.run(
                [
                    "curl",
                    "-s",
                    "-H",
                    "Content-Type: application/json",
                    "-d",
                    json.dumps(data),
                    self.url,
                ],
                capture_output=True,
                text=True,
                check=True,
            )
            
            logger.debug(f"Raw curl output - stdout: {result.stdout}")
            if result.stderr:
                logger.warning(f"curl stderr output: {result.stderr}")
            
            try:
                response = json.loads(result.stdout)
                logger.debug(f"Parsed JSON response: {json.dumps(response, indent=2)}")
                
                if "error" in response:
                    logger.error(f"RPC call returned error response: {json.dumps(response['error'], indent=2)}")
                    return response
                    
                if "result" in response:
                    logger.info(f"RPC call successful with result type: {type(response['result'])}")
                    logger.debug(f"RPC result: {json.dumps(response['result'], indent=2)}")
                else:
                    logger.warning("RPC response missing 'result' field")
                    
                return response
                
            except json.JSONDecodeError as e:
                logger.error(f"Failed to parse JSON response: {e}")
                logger.error(f"Raw response that failed parsing: {result.stdout}")
                logger.error(f"JSON parse error details - line: {e.lineno}, column: {e.colno}, message: {e.msg}")
                raise PartnerChainRpcException(f"Failed to parse JSON response: {e}")

        except subprocess.CalledProcessError as e:
            logger.error(f"RPC call failed with return code: {e.returncode}")
            logger.error(f"Command that failed: {' '.join(e.cmd)}")
            logger.error(f"stdout: {e.stdout}")
            logger.error(f"stderr: {e.stderr}")
            raise PartnerChainRpcException(f"RPC call failed: {e}")

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
