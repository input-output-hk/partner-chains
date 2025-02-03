import logging
from config.api_config import ApiConfig


class PartnerChainEpochCalculator:
    def __init__(self, config: ApiConfig):
        self.config = config

    def get_mc_epoch_change_timestamp(self, mc_epoch) -> int:
        return (
            mc_epoch * self.config.main_chain.epoch_length * self.config.main_chain.slot_length
            + self.config.main_chain.init_timestamp
        )

    def get_first_pc_epoch(self, mc_epoch) -> int:
        pc_epoch = self.get_mc_epoch_change_timestamp(mc_epoch) / (
            self.config.nodes_config.block_duration * self.config.nodes_config.slots_in_epoch
        )
        return int(pc_epoch)

    def find_pc_epochs(self, mc_epoch, start_from_initial_pc_epoch=True):
        """Returns pc epochs range for given mc_epoch.

        Arguments:
            mc_epoch {int} -- Main Chain epoch

        Keyword Arguments:
            start_from_initial_pc_epoch {bool} -- If sidechain was initialized in given mc_epoch return range from
            initial pc epoch (default: {True})

        Returns:
            range -- Python range object with sc epochs that belong to given mc_epoch
        """
        if start_from_initial_pc_epoch and mc_epoch == self.config.deployment_mc_epoch:
            first_pc_epoch = self.config.initial_pc_epoch
            logging.info(
                "This is the deployment MC epoch, so the PC epoch range will start from initial PC epoch "
                f"{first_pc_epoch}."
            )
        else:
            first_pc_epoch = self.get_first_pc_epoch(mc_epoch)
        first_pc_epoch_of_next_mc_epoch = self.get_first_pc_epoch(mc_epoch + 1)
        epochs_range = range(first_pc_epoch, first_pc_epoch_of_next_mc_epoch)  # range is [a,b) object
        logging.info(
            f"PC epochs range of MC epoch {mc_epoch} is {self.range_in_math_notation(epochs_range)}, both included. "
            f"It's {len(epochs_range)} epochs."
        )
        return epochs_range

    def find_mc_epoch(self, pc_epoch, current_mc_epoch):
        for mc_epoch in range(current_mc_epoch, 0, -1):
            first_pc_epoch = self.get_first_pc_epoch(mc_epoch)
            last_pc_epoch = self.get_first_pc_epoch(mc_epoch + 1)
            if first_pc_epoch <= pc_epoch < last_pc_epoch:
                return mc_epoch
        return None

    def range_in_math_notation(self, range):
        """Present python range object accordingly to math's interval notation."""
        return f"[{range.start},{range.stop - 1}]"
