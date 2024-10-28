pub use partner_chains_plutus_data::d_param::DParamDatum;
pub use permissioned::PermissionedCandidateDatums;
pub use registered::*;

mod permissioned;
mod registered;

type Error = Box<dyn std::error::Error + Send + Sync>;
type Result<T> = std::result::Result<T, Error>;
