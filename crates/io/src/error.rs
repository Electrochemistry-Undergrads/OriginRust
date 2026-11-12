//! The one error type of this crate. Every message names the file (or the
//! import source) so that it can be shown to the user as it is.

use std::path::PathBuf;

/// Everything that can go wrong while reading or writing files.
#[derive(Debug, thiserror::Error)]
pub enum IoError {
    /// The operating system refused to read or write a file.
    #[error("cannot read or write '{}': {source}", path.display())]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    /// The file's bytes cannot be read as text (binary data, broken UTF-16).
    #[error("'{}' cannot be read as text: {reason}", path.display())]
    NotText { path: PathBuf, reason: String },

    /// The file is text but holds no usable data rows.
    #[error("'{source_name}' contains no data rows: {detail}")]
    NoData { source_name: String, detail: String },

    /// The file was expected to be a CH Instruments export but is not.
    #[error(
        "'{source_name}' is not a CH Instruments text file: no column header such as \
         'Potential/V, Current/A' was found in the first 100000 lines"
    )]
    NotChi { source_name: String },

    /// A CH Instruments binary file (`.bin`) that cannot be read: damaged, cut short, or of a
    /// technique whose layout is not known.
    #[error("'{source_name}' is a CH Instruments binary file that cannot be read: {detail}")]
    ChiBinary { source_name: String, detail: String },

    /// An import option has a value that cannot work.
    #[error("invalid import option: {0}")]
    InvalidOption(String),

    /// The ZIP container of a project file is damaged or cannot be written.
    #[error("'{}' is not a readable project archive: {source}", path.display())]
    Zip {
        path: PathBuf,
        #[source]
        source: zip::result::ZipError,
    },

    /// A JSON member of a project file cannot be read or written.
    #[error("'{}': member '{member}' is not valid JSON: {source}", path.display())]
    Json {
        path: PathBuf,
        member: String,
        #[source]
        source: serde_json::Error,
    },

    /// A ZIP archive that is not a Sheaf project.
    #[error("'{}' is not a Sheaf project file (its manifest does not say 'sheaf-project')", path.display())]
    NotAProject { path: PathBuf },

    /// The project was saved by a newer program than this one.
    #[error(
        "'{}' uses project file format version {found}, but this program only reads versions up to \
         {supported}; please update Sheaf to open it",
        path.display()
    )]
    UnsupportedVersion { path: PathBuf, found: u32, supported: u32 },

    /// The project description lists a column whose data is not in the archive.
    #[error("'{}' is damaged: the data of column {column} is missing from the archive", path.display())]
    MissingColumnData { path: PathBuf, column: String },

    /// A member of a project file has an impossible content.
    #[error("'{}' is damaged: member '{member}' {detail}", path.display())]
    Corrupt { path: PathBuf, member: String, detail: String },

    /// The file is not an Origin project (.opj) that can be read.
    #[error("'{}' is not a readable Origin project (.opj): {reason}", path.display())]
    OpjFormat { path: PathBuf, reason: String },

    /// Two columns of a project share one identifier, so their data would collide.
    #[error("'{}' cannot be saved: column id {id} is used by more than one column", path.display())]
    DuplicateColumnId { path: PathBuf, id: u64 },
}

impl IoError {
    pub(crate) fn io(path: &std::path::Path, source: std::io::Error) -> Self {
        IoError::Io { path: path.to_path_buf(), source }
    }
}
