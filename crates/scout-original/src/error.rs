use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("cannot read {path}: {source}")]
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("not an HFS disk image")]
    NotHfs,
    #[error("file {0:?} not found on the disk")]
    FileNotFound(String),
    #[error("resource '{kind}' {id} not found")]
    MissingResource { kind: String, id: i16 },
    #[error("corrupt data: {0}")]
    Corrupt(String),
    #[error(transparent)]
    Data(#[from] scout_data::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

pub(crate) fn corrupt(message: impl Into<String>) -> Error {
    Error::Corrupt(message.into())
}
