# Block Participation

This crate implements the logic that provides information on block producers and their
delegators in the form of inherent data.


## Usage

This feature's scope is providing inherent data with information on block production only,
in the form of a `BlockProductionData` instance.
Each Partner Chain should implement its own pallet `HandlerPallet` to handle this data
in accordance to its business needs and ledger logic.

The components in this crate are meant to work together with the
Block Participation pallet (see `toolkit/pallets/block-participation`) and
Block Production Log pallet (see `toolkit/pallets/block-production-log/readme.md`).

Block Participation pallet should be included in the runtime and configured with:
- `DelegatorId` type. If the Cardano stake-based IDP defined in this crate is to be used,
  this ID type should be obtainable from `sidechain_domain::DelegatorKey` through `From` trait.
- `should_release_data` - function, which should return `None` or the slot number up to
  which the block participation data should be computed
- `TARGET_INHERENT_ID` - inherent identifier consumed by `HandlerPallet`

Block Production Log pallet is used by the Block Participation pallet as the source of
block production data and configures the `BlockProducerId` type, which must implement
`AsCardanoSPO` trait, which provides an optional cast to `sidechain_domain::MainchainKeyHash`.

## Process

``` mermaid
sequenceDiagram
	participant ProductionLog as Block Production Log Pallet
	participant BlockParticipationIDP as Block Participation IDP<br>(inherent data provider)
	participant BlockParticipationPallet as Block Participation Pallet

	loop Every block
		ProductionLog ->> ProductionLog: Record block producer
	end
	BlockParticipationIDP ->> BlockParticipationPallet: Check if data should be produced
	BlockParticipationPallet -->> BlockParticipationIDP: SLOT: expected data upper bound
	BlockParticipationIDP ->> ProductionLog: Fetch block production data up to SLOT<br>(Runtime API call)
	ProductionLog -->> BlockParticipationIDP: PRODUCTION DATA
	BlockParticipationIDP ->> DataSources: Query stake distribution
	DataSources -->> BlockParticipationIDP: STAKE DISTRIBUTION
	BlockParticipationIDP ->> BlockParticipationIDP: Join PRODUCTION DATA with STAKE DISTRIBUTION<br>creating BLOCK PARTICIPATION DATA
	BlockParticipationIDP -->> HandlerPallet: Provide BLOCK PARTICIPATION DATA<br>(inherent data)
	HandlerPallet ->> HandlerPallet: Handle BLOCK PARTICIPATION DATA (eg. payouts)<br>(inherent)
	BlockParticipationIDP -->> BlockParticipationPallet: Provide SLOT<br>(inherent data)
	BlockParticipationPallet ->> ProductionLog: Clear production log up to SLOT<br>(inherent)
```

