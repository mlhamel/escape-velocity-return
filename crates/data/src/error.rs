use std::collections::HashSet;
use std::fmt;
use std::path::PathBuf;

#[derive(Debug)]
pub enum ContentError {
    Io(std::io::Error),
    Parse { path: PathBuf, message: String },
    DuplicateId { kind: &'static str, id: String },
    UnknownReference { kind: &'static str, id: String },
}

impl fmt::Display for ContentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(err) => write!(f, "io error: {err}"),
            Self::Parse { path, message } => {
                write!(f, "parse error in {}: {message}", path.display())
            }
            Self::DuplicateId { kind, id } => write!(f, "duplicate {kind} id: {id}"),
            Self::UnknownReference { kind, id } => write!(f, "unknown {kind} reference: {id}"),
        }
    }
}

impl std::error::Error for ContentError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(err) => Some(err),
            _ => None,
        }
    }
}

impl From<std::io::Error> for ContentError {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err)
    }
}

pub fn validate_ids(
    kind: &'static str,
    ids: impl Iterator<Item = String>,
) -> Result<(), ContentError> {
    let mut seen = HashSet::new();
    for id in ids {
        if !seen.insert(id.clone()) {
            return Err(ContentError::DuplicateId { kind, id });
        }
    }
    Ok(())
}

pub fn validate_reference(
    kind: &'static str,
    id: &str,
    known: &HashSet<String>,
) -> Result<(), ContentError> {
    if known.contains(id) {
        Ok(())
    } else {
        Err(ContentError::UnknownReference {
            kind,
            id: id.to_owned(),
        })
    }
}

pub fn validate_asset(assets_root: &std::path::Path, sprite: &str) -> Result<(), ContentError> {
    if assets_root.join(sprite).is_file() {
        Ok(())
    } else {
        Err(ContentError::UnknownReference {
            kind: "asset",
            id: sprite.to_owned(),
        })
    }
}
