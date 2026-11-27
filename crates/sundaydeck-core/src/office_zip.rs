//! An opened Office (`.pptx`/`.docx`) archive whose XML part reads share one
//! [`OfficeReadBudget`], so the total XML read from one archive stays bounded.

use std::io::Cursor;

use zip::ZipArchive;

use crate::error::ImportError;
use crate::zip_limits::{OfficeReadBudget, open_archive_checked};

pub(crate) struct OfficeZip<'a> {
    archive: ZipArchive<Cursor<&'a [u8]>>,
    budget: OfficeReadBudget,
}

impl<'a> OfficeZip<'a> {
    pub(crate) fn open(bytes: &'a [u8]) -> Result<Self, ImportError> {
        Ok(Self {
            archive: open_archive_checked(Cursor::new(bytes), "invalid Office archive")?,
            budget: OfficeReadBudget::default(),
        })
    }

    /// Entry names in central-directory order (names that fail to decode are skipped).
    pub(crate) fn entry_names(&self) -> impl Iterator<Item = String> + '_ {
        self.archive
            .file_names()
            .filter_map(|name| name.ok().map(|n| n.into_owned()))
    }

    pub(crate) fn has_entry(&self, name: &str) -> bool {
        self.archive
            .file_names()
            .any(|n| n.is_ok_and(|s| s == name))
    }

    /// Reads XML part `entry_name` as UTF-8, charged against the archive's read budget;
    /// `Ok(None)` when the part is absent.
    pub(crate) fn read_text(&mut self, entry_name: &str) -> Result<Option<String>, ImportError> {
        self.budget
            .read(&mut self.archive, entry_name)?
            .map(|bytes| {
                String::from_utf8(bytes).map_err(|err| ImportError::InvalidArchive {
                    detail: format!("failed reading {entry_name}: {err}"),
                })
            })
            .transpose()
    }
}
