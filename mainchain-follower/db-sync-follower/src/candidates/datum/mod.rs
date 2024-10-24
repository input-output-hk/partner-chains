pub use d_param::DParamDatum;
pub use permissioned::PermissionedCandidateDatums;
pub use registered::*;

mod d_param;
mod permissioned;
mod registered;

type Error = Box<dyn std::error::Error + Send + Sync>;
type Result<T> = std::result::Result<T, Error>;

trait PlutusDataExtensions {
	fn as_u16(self) -> Option<u16>;
}

impl PlutusDataExtensions for cardano_serialization_lib::PlutusData {
	fn as_u16(self) -> Option<u16> {
		u16::try_from(u32::try_from(self.as_integer()?.as_u64()?).ok()?).ok()
	}
}
