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
            
            try:
                return json.loads(result.stdout)
            except json.JSONDecodeError as e:
                logging.error(f"Failed to parse JSON response: {e}")
                logging.error(f"Raw response: {result.stdout}")
                raise PartnerChainRpcException(f"Failed to parse JSON response: {e}")

        except subprocess.CalledProcessError as e:
            logging.error(f"RPC call failed: {e}")
            logging.error(f"stdout: {e.stdout}")
            logging.error(f"stderr: {e.stderr}")
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