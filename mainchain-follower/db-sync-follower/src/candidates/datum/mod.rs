pub use d_param::DParamDatum;
pub use permissioned::PermissionedCandidateDatums;

pub use registered::*;

mod d_param;
mod permissioned;
mod registered;

type Error = Box<dyn std::error::Error + Send + Sync>;
type Result<T> = std::result::Result<T, Error>;
