from sqlalchemy import String, BigInteger
from sqlalchemy.orm import DeclarativeBase, Mapped, mapped_column
from typing import Optional


class Base(DeclarativeBase):
    pass


class Tx(Base):
    __tablename__ = "tx"
    id: Mapped[int] = mapped_column(primary_key=True)
    hash: Mapped[Optional[str]] = mapped_column(String(128))
    block_id: Mapped[int] = mapped_column(BigInteger)
    block_index: Mapped[str] = mapped_column(String(128))
    out_sum: Mapped[int] = mapped_column(BigInteger)
    fee: Mapped[int] = mapped_column(BigInteger)
    deposit: Mapped[int] = mapped_column(BigInteger)
    size: Mapped[str] = mapped_column(String(128))
    invalid_before: Mapped[str] = mapped_column(String(128))
    invalid_hereafter: Mapped[str] = mapped_column(String(128))
    valid_contract: Mapped[bool] = mapped_column(default=False)
    script_size: Mapped[str] = mapped_column(String(128))

    def __repr__(self) -> str:
        return (
            f"Tx(id={self.id!r}, hash={self.hash!r}, block_id={self.block_id!r}, block_index={self.block_index!r}, "
            f"out_sum={self.out_sum!r}, out_sum={self.out_sum!r}))"
        )


class Block(Base):
    __tablename__ = "block"
    id: Mapped[int] = mapped_column(primary_key=True)
    hash: Mapped[str] = mapped_column(String(128))
    epoch_no: Mapped[str] = mapped_column(String(128))
    slot_no: Mapped[str] = mapped_column(String(128))
    epoch_slot_no: Mapped[str] = mapped_column(String(128))
    block_no: Mapped[str] = mapped_column(String(128))
    previous_id: Mapped[int]
    slot_leader_id: Mapped[int]
    size: Mapped[str] = mapped_column(String(128))
    time: Mapped[str] = mapped_column(String(128))
    tx_count: Mapped[int] = mapped_column(BigInteger)
    proto_major: Mapped[str] = mapped_column(String(128))
    proto_minor: Mapped[str] = mapped_column(String(128))
    vrf_key: Mapped[str] = mapped_column(String(128))
    op_cert: Mapped[str] = mapped_column(String(128))
    op_cert_counter: Mapped[str] = mapped_column(String(128))

    def __repr__(self) -> str:
        return f"BlockId(id={self.id!r}, hash={self.hash!r}, epoch_no={self.epoch_no!r}, " f"slot_no={self.slot_no!r})"
