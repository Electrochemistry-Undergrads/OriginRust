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

    /// A window file (File > Save Window As) was given where a project is needed.
    #[error("'{}' is a window file (one workbook or graph), not a project; open it to add its window to the open project", path.display())]
    IsAWindowFile { path: PathBuf },

    /// A file that is not a window file was given where one is needed.
    #[error("'{}' is not a Sheaf window file (it is a project, or has no window marker in its manifest)", path.display())]
    NotAWindowFile { path: PathBuf },

    /// The window to save is not in the project.
    #[error("cannot save the window: the project has no window with id {id}")]
    NoSuchWindow { id: u64 },

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

    /// The file is not an Origin Unicode project (.opju) that can be read.
    #[error("'{}' is not a readable Origin Unicode project (.opju): {reason}", path.display())]
    OpjuFormat { path: PathBuf, reason: String },

    /// A save was asked for in the Origin Unicode format, which Sheaf reads but cannot write.
    #[error(
        "'{}' was not saved: Sheaf reads Origin Unicode projects (.opju) but cannot write them; save as .opj, which Origin 2018 and later open as well",
        path.display()
    )]
    OpjuWrite { path: PathBuf },

    /// The project holds something that an Origin project cannot store, or a name Origin would refuse.
    #[error("cannot write an Origin project (.opj): {reason}")]
    OpjWrite { reason: String },

    /// A file named like the companion exists and Sheaf did not write it, so it is not replaced.
    #[error("'{}' already exists and is not a companion file written by Sheaf, so it was not replaced; move or rename it and save again", path.display())]
    CompanionExists { path: PathBuf },

    /// An Excel workbook (xls, xlsx, xlsm) that cannot be read.
    #[error("'{}' is not a readable Excel workbook: {reason}", path.display())]
    ExcelRead { path: PathBuf, reason: String },

    /// An Excel workbook (xlsx) that cannot be written.
    #[error("cannot write the Excel workbook '{}': {reason}", path.display())]
    ExcelWrite { path: PathBuf, reason: String },

    /// An Excel export would replace a file that is not an Excel workbook.
    #[error("'{}' already exists and is not an Excel (.xlsx) workbook, so it was not replaced; choose another name", path.display())]
    ExportExists { path: PathBuf },

    /// Two columns of a project share one identifier, so their data would collide.
    #[error("'{}' cannot be saved: column id {id} is used by more than one column", path.display())]
    DuplicateColumnId { path: PathBuf, id: u64 },
}

impl IoError {
    pub(crate) fn io(path: &std::path::Path, source: std::io::Error) -> Self {
        IoError::Io { path: path.to_path_buf(), source }
    }
}
