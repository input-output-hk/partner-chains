from sqlalchemy import String, BigInteger
from sqlalchemy.orm import DeclarativeBase, Mapped, mapped_column
from typing import Optional


class Base(DeclarativeBase):
    pass


class Candidates(Base):
    __tablename__ = "candidates"
    id: Mapped[int] = mapped_column(primary_key=True)
    name: Mapped[str] = mapped_column(String(100))
    next_status: Mapped[Optional[str]] = mapped_column(String(100))
    next_status_epoch: Mapped[Optional[int]]

    def __repr__(self) -> str:
        return (
            f"Candidate(id={self.id!r}, name={self.name!r}, "
            f"next_status={self.next_status!r}), next_status_epoch={self.next_status_epoch!r})"
        )


class PermissionedCandidates(Base):
    __tablename__ = "permissioned_candidates"
    id: Mapped[int] = mapped_column(primary_key=True)
    name: Mapped[str] = mapped_column(String(100))
    next_status: Mapped[Optional[str]] = mapped_column(String(100))
    next_status_epoch: Mapped[Optional[int]]

    def __repr__(self) -> str:
        return (
            f"Permissioned candidate(id={self.id!r}, name={self.name!r}, "
            f"next_status={self.next_status!r}), next_status_epoch={self.next_status_epoch!r})"
        )


class IncomingTx(Base):
    __tablename__ = "incoming_txs"
    id: Mapped[int] = mapped_column(primary_key=True)
    pc_addr: Mapped[str] = mapped_column(String(128))
    mc_addr: Mapped[str] = mapped_column(String(128))
    pc_balance: Mapped[int] = mapped_column(BigInteger)
    mc_balance: Mapped[int]
    token_policy_id: Mapped[str] = mapped_column(String(128))
    amount: Mapped[int]
    stable_at_block: Mapped[Optional[int]]
    tx_hash: Mapped[Optional[str]] = mapped_column(String(128))
    is_settled: Mapped[bool] = mapped_column(default=False)
    pc_balance_after_settlement: Mapped[Optional[int]] = mapped_column(BigInteger)

    def __repr__(self) -> str:
        return (
            f"IncomingTx(id={self.id!r}, dest={self.pc_addr!r}, dest_balance={self.pc_balance!r}, "
            f"amount={self.amount!r})"
        )


class OutgoingTx(Base):
    __tablename__ = "outgoing_txs"
    id: Mapped[int] = mapped_column(primary_key=True)
    pc_addr: Mapped[str] = mapped_column(String(128))
    mc_addr: Mapped[str] = mapped_column(String(128))
    pc_balance: Mapped[int] = mapped_column(BigInteger)
    mc_balance: Mapped[int]
    amount: Mapped[int] = mapped_column(BigInteger)
    pc_balance_after_lock: Mapped[Optional[int]] = mapped_column(BigInteger)
    fees_spent: Mapped[Optional[int]] = mapped_column(BigInteger)
    available_on_pc_epoch: Mapped[Optional[int]]
    tx_index_on_pc_epoch: Mapped[Optional[int]]
    token_policy_id: Mapped[str] = mapped_column(String(128))
    lock_tx_hash: Mapped[Optional[str]] = mapped_column(String(128))
    combined_proof: Mapped[Optional[str]]
    mc_balance_before_claim: Mapped[Optional[int]] = mapped_column(BigInteger)
    mc_balance_after_claim: Mapped[Optional[int]] = mapped_column(BigInteger)
    is_claimed: Mapped[bool] = mapped_column(default=False)
    is_received: Mapped[Optional[bool]]

    def __repr__(self) -> str:
        return (
            f"OutgoingTx(id={self.id!r}, dest={self.mc_addr!r}, dest_balance={self.mc_balance!r}, "
            f"amount={self.amount!r})"
        )


class StakeDistributionCommittee(Base):
    __tablename__ = "stake_distribution_committee"
    id: Mapped[int] = mapped_column(primary_key=True)
    mc_epoch: Mapped[int]
    mc_vkey: Mapped[str] = mapped_column(String(128))
    sc_pub_key: Mapped[Optional[str]] = mapped_column(String(128))
    pc_pub_key: Mapped[str] = mapped_column(String(128))
    pool_id: Mapped[Optional[str]] = mapped_column(String(128))
    stake_delegation: Mapped[Optional[int]] = mapped_column(BigInteger)
    probability: Mapped[Optional[float]] = mapped_column(default=0.0)
    actual_attendance: Mapped[Optional[int]]
    expected_attendance: Mapped[Optional[float]]

    def __repr__(self) -> str:
        return (
            f"StakeDistributionCommittee(id={self.id!r}, mc_epoch={self.mc_epoch!r}, "
            f"mc_vkey={self.mc_vkey!r}, pc_pub_key={self.pc_pub_key!r}, attendance={self.actual_attendance!r})"
        )
