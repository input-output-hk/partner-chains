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
