//! Stable identifiers. They are unique within one project and never reused,
//! so a graph can keep pointing at a column after columns are inserted,
//! moved or renamed.

use serde::{Deserialize, Serialize};

/// Identifies one column anywhere in a project.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ColumnId(pub u64);

/// Identifies one window (a workbook or a graph) in a project.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct WindowId(pub u64);
