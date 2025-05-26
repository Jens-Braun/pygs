use thiserror::Error;

#[derive(Error, Debug)]
pub(crate) enum BLHAError {
    #[error("Error while accessing {0}: {1}")]
    IOError(String, #[source] std::io::Error),
    #[error("OLP returned from function {0} with error code {1}")]
    OLPError(String, i32),
    #[error("OLP did not process the order file successfully: {0}")]
    ContractError(String),
    #[error("Error while parsing {0}: {1}")]
    ParseError(String, #[source] peg::error::ParseError<peg::str::LineCol>),
    #[error("Error while loading shared library: {0}")]
    LibraryError(#[from] libloading::Error),
}
