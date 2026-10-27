use crate::Worksheet;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashSet,
    fmt, fs,
    io::{self, Read, Write},
    path::Path,
};

const MAX_BYTES: u64 = 8 * 1024 * 1024;
const MAGIC: &str = "quire-worksheet";
const VERSION: u32 = 1;

#[derive(Debug)]
pub struct FileError(String);

impl fmt::Display for FileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for FileError {}
impl From<io::Error> for FileError {
    fn from(value: io::Error) -> Self {
        Self(format!("File operation failed: {value}"))
    }
}
impl From<serde_json::Error> for FileError {
    fn from(value: serde_json::Error) -> Self {
        Self(format!("Invalid Quire JSON: {value}"))
    }
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Document {
    format: String,
    version: u32,
    worksheet: Worksheet,
}

fn validate(sheet: &Worksheet) -> Result<(), FileError> {
    let mut ids = HashSet::new();
    for region in &sheet.regions {
        if region.id == 0 || region.id >= u64::MAX - 1 || !ids.insert(region.id) {
            return Err(FileError(
                "Region IDs must be unique, nonzero, and below u64::MAX - 1".into(),
            ));
        }
    }
    Ok(())
}

fn check_path(path: &Path) -> Result<(), FileError> {
    if !path
        .file_name()
        .and_then(|s| s.to_str())
        .is_some_and(|s| s.to_ascii_lowercase().ends_with(".quire.json"))
    {
        return Err(FileError("Only .quire.json files are supported. Mathcad files cannot be opened or saved; compatibility is unverified.".into()));
    }
    Ok(())
}

impl Worksheet {
    /// Versioned Quire-owned JSON. Unknown fields/kinds/versions are rejected
    /// rather than silently lost by a later save. Math source is stored verbatim.
    pub fn to_json(&self) -> Result<String, FileError> {
        validate(self)?;
        let json = serde_json::to_string_pretty(&Document {
            format: MAGIC.into(),
            version: VERSION,
            worksheet: self.clone(),
        })?;
        if json.len() as u64 > MAX_BYTES {
            return Err(FileError("Worksheet exceeds the 8 MiB file limit".into()));
        }
        Ok(json)
    }

    pub fn from_json(json: &str) -> Result<Self, FileError> {
        if json.len() as u64 > MAX_BYTES {
            return Err(FileError("Worksheet exceeds the 8 MiB file limit".into()));
        }
        let document: Document = serde_json::from_str(json)?;
        if document.format != MAGIC || document.version != VERSION {
            return Err(FileError(
                "Unsupported file format or version; expected quire-worksheet version 1".into(),
            ));
        }
        validate(&document.worksheet)?;
        Ok(document.worksheet)
    }

    pub fn open(path: impl AsRef<Path>) -> Result<Self, FileError> {
        let path = path.as_ref();
        check_path(path)?;
        let mut data = String::new();
        fs::File::open(path)?
            .take(MAX_BYTES + 1)
            .read_to_string(&mut data)?;
        Self::from_json(&data)
    }

    /// The destination changes only after serialization, validation, writing,
    /// flushing and syncing a temporary file in the same directory succeed.
    /// A failed replacement leaves the old file intact. This does not promise
    /// power-loss durability of the directory entry on every filesystem.
    pub fn save(&self, path: impl AsRef<Path>) -> Result<(), FileError> {
        let path = path.as_ref();
        check_path(path)?;
        let json = self.to_json()?;
        atomic_save(path, |file| file.write_all(json.as_bytes()))
    }
}

fn atomic_save(
    path: &Path,
    write: impl FnOnce(&mut fs::File) -> io::Result<()>,
) -> Result<(), FileError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) => {
            if !metadata.file_type().is_file() || metadata.permissions().readonly() {
                return Err(FileError(
                    "Destination must be a writable regular file (symbolic links are refused)"
                        .into(),
                ));
            }
        }
        Err(e) if e.kind() == io::ErrorKind::NotFound => (),
        Err(e) => return Err(e.into()),
    }
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let mut temp = tempfile::NamedTempFile::new_in(parent)?;
    write(temp.as_file_mut())?;
    temp.as_file_mut().flush()?;
    temp.as_file().sync_all()?;
    temp.persist(path).map_err(|e| {
        FileError(format!(
            "Cannot replace destination; original kept: {}",
            e.error
        ))
    })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interrupted_write_keeps_existing_file_and_cleans_temporary_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("existing.quire.json");
        fs::write(&path, "original content").unwrap();
        let failure = atomic_save(&path, |file| {
            file.write_all(b"incomplete replacement")?;
            Err(io::Error::other("simulated disk full"))
        });
        assert!(failure.is_err());
        assert_eq!(fs::read_to_string(path).unwrap(), "original content");
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1);
    }

    #[test]
    fn failed_replace_keeps_destination() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("target.quire.json");
        fs::create_dir(&path).unwrap();
        assert!(Worksheet::default().save(&path).is_err());
        assert!(path.is_dir());
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1);
    }
}
