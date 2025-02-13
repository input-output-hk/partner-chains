import logging
from config.api_config import ApiConfig
from .blockchain_api import BlockchainApi

MAX_MISSING_BLOCKS_PER_PC_EPOCH = 10
BLOCKS_AFTER_EPOCH_CHANGE_FROM_NEW_COMMITTEE = 1


class BlockFinder:
    def __init__(self, api: BlockchainApi, config: ApiConfig):
        self.config = config
        self.api = api

    def get_block_with_timestamp(
        self, start_search_block_no: int, end_search_block_no: int, target_timestamp_seconds: int
    ):
        middle_block_no = int((start_search_block_no + end_search_block_no) / 2)
        block_timestamp = self.api.get_block_extrinsic_value("Timestamp", middle_block_no)
        block_timestamp_seconds = int(block_timestamp / 1000)
        if block_timestamp_seconds == target_timestamp_seconds:
            return middle_block_no
        elif end_search_block_no - start_search_block_no == 1:
            next_block_timestamp = self.api.get_block_extrinsic_value("Timestamp", end_search_block_no)
            next_block_timestamp_seconds = int(next_block_timestamp / 1000)
            if next_block_timestamp_seconds == target_timestamp_seconds:
                return end_search_block_no
            else:
                logging.error(f"Missing block with timestamp {target_timestamp_seconds}.")
                return None
        elif block_timestamp_seconds < target_timestamp_seconds:
            return self.get_block_with_timestamp(middle_block_no, end_search_block_no, target_timestamp_seconds)
        elif block_timestamp_seconds > target_timestamp_seconds:
            return self.get_block_with_timestamp(start_search_block_no, middle_block_no, target_timestamp_seconds)
        else:
            logging.error(f"Missing block with timestamp {target_timestamp_seconds}.")
            return None

    def get_block_range(self, next_epoch_timestamp, current_pc_epoch, pc_epoch):
        logging.info(f"Finding block range for epoch {pc_epoch}...")
        epoch_to_test_start_timestamp = (
            next_epoch_timestamp
            - (current_pc_epoch - pc_epoch + 1)
            * self.config.nodes_config.block_duration
            * self.config.nodes_config.slots_in_epoch
            * 1000
        )
        epoch_to_test_end_timestamp = epoch_to_test_start_timestamp + (
            self.config.nodes_config.block_duration
            * self.config.nodes_config.slots_in_epoch
            * 1000  # secs to millisecs
        )

        latest_block_number = self.api.get_latest_pc_block_number()
        latest_block_timestamp = self.api.get_block_extrinsic_value("Timestamp", latest_block_number)
        time_diff = int((latest_block_timestamp - epoch_to_test_start_timestamp) / 1000)  # millisecs to secs
        approximate_blocks_in_time_diff = int(time_diff / self.config.nodes_config.block_duration)
        approximate_first_block = latest_block_number - approximate_blocks_in_time_diff
        if approximate_first_block < 0:
            approximate_first_block = 0

        epoch_to_test_start_timestamp_seconds = int(epoch_to_test_start_timestamp / 1000)
        epoch_to_test_end_timestamp_seconds = int(epoch_to_test_end_timestamp / 1000)

        retries = 0
        first_block_number = None
        while retries < MAX_MISSING_BLOCKS_PER_PC_EPOCH and not first_block_number:
            first_block_number = self.get_block_with_timestamp(
                approximate_first_block,
                approximate_first_block + MAX_MISSING_BLOCKS_PER_PC_EPOCH * (current_pc_epoch - pc_epoch),
                epoch_to_test_start_timestamp_seconds,
            )
            epoch_to_test_start_timestamp_seconds += self.config.nodes_config.block_duration
            retries += 1

        if not first_block_number:
            logging.error(
                f"Missing first block of epoch {pc_epoch} for timestamp {epoch_to_test_start_timestamp_seconds}."
            )
            return None

        first_block_number += BLOCKS_AFTER_EPOCH_CHANGE_FROM_NEW_COMMITTEE

        retries = 0
        last_block_number = None
        while retries < MAX_MISSING_BLOCKS_PER_PC_EPOCH and not last_block_number:
            last_block_number = self.get_block_with_timestamp(
                first_block_number + self.config.nodes_config.slots_in_epoch - MAX_MISSING_BLOCKS_PER_PC_EPOCH,
                first_block_number + self.config.nodes_config.slots_in_epoch + MAX_MISSING_BLOCKS_PER_PC_EPOCH,
                epoch_to_test_end_timestamp_seconds,
            )
            epoch_to_test_end_timestamp_seconds += self.config.nodes_config.block_duration
            retries += 1

        if not last_block_number:
            logging.error(
                f"Missing last block of epoch {pc_epoch} for timestamp {epoch_to_test_end_timestamp_seconds}."
            )
            return None
        return range(first_block_number, last_block_number + 1)
