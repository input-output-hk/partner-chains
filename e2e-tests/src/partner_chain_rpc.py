import requests
import logging as logger
from dataclasses import dataclass
from pydantic import BaseModel
from typing import Optional


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

    def partner_chain_get_epoch_committee(self, epoch) -> PartnerChainRpcResponse:
        response = requests.post(
            self.url, headers=self.headers, json=self.__get_body(method="sidechain_getEpochCommittee", params=[epoch])
        )
        json_data = response.json()
        logger.debug(json_data)
        return PartnerChainRpcResponse.model_validate(json_data)

    def partner_chain_get_status(self):
        response = requests.post(self.url, headers=self.headers, json=self.__get_body(method="sidechain_getStatus"))
        json_data = response.json()
        logger.debug(json_data)
        return PartnerChainRpcResponse.model_validate(json_data)

    def partner_chain_get_ariadne_parameters(self, mc_epoch) -> PartnerChainRpcResponse:
        response = requests.post(
            self.url,
            headers=self.headers,
            json=self.__get_body(method="sidechain_getAriadneParameters", params=[mc_epoch]),
        )
        json_data = response.json()
        logger.debug(json_data)
        return PartnerChainRpcResponse.model_validate(json_data)

    def partner_chain_get_params(self):
        response = requests.post(self.url, headers=self.headers, json=self.__get_body(method="sidechain_getParams"))
        json_data = response.json()
        logger.debug(json_data)
        return PartnerChainRpcResponse.model_validate(json_data)

    def partner_chain_get_registrations(self, mc_epoch, mc_key):
        response = requests.post(
            self.url,
            headers=self.headers,
            json=self.__get_body(method="sidechain_getRegistrations", params=[mc_epoch, mc_key]),
        )
        json_data = response.json()
        logger.debug(json_data)
        return PartnerChainRpcResponse.model_validate(json_data)
