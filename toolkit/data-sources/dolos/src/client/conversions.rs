use crate::{DataSourceError, Result};
use blockfrost_openapi::models::block_content::BlockContent;
use sidechain_domain::*;

#[cfg(feature = "bridge")]
use {
	blockfrost_openapi::models::{
		tx_content_metadata_inner::TxContentMetadataInner,
		tx_content_metadata_inner_json_metadata::TxContentMetadataInnerJsonMetadata,
	},
	cardano_serialization_lib::{MetadataMap, TransactionMetadatum},
};

pub fn from_block_content(value: BlockContent) -> Result<MainchainBlock> {
	Ok(MainchainBlock {
		number: value
			.height
			.map(|n| sidechain_domain::McBlockNumber(n as u32))
			.ok_or(DataSourceError::InvalidData("number missing".to_string()))?,
		hash: McBlockHash::decode_hex(&value.hash)?,
		epoch: value
			.epoch
			.map(|n| sidechain_domain::McEpochNumber(n as u32))
			.ok_or(DataSourceError::InvalidData("epoch missing".to_string()))?,
		slot: value
			.slot
			.map(|n| sidechain_domain::McSlotNumber(n as u64))
			.ok_or(DataSourceError::InvalidData("slot missing".to_string()))?,
		timestamp: value.time as u64,
	})
}

#[cfg(feature = "bridge")]
pub(crate) fn metadata_from_response(
	value: TxContentMetadataInner,
) -> Result<TransactionMetadatum> {
	let result = match *value.json_metadata.to_owned() {
		TxContentMetadataInnerJsonMetadata::String(s) => TransactionMetadatum::new_text(s)?,
		TxContentMetadataInnerJsonMetadata::Object(obj) => {
			let mut map = MetadataMap::new();
			for (key, value) in obj {
				let value = serde_json::from_value(value)?;
				map.insert_str(&key, &value)?;
			}

			TransactionMetadatum::new_map(&map)
		},
	};

	Ok(result)
}
