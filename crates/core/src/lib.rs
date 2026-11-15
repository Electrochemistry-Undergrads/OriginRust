//! Document model of the application.
//!
//! This crate has no GUI and no file-format code. It defines what a project
//! *is*: a set of workbooks (tables of columns) and graphs (figure
//! descriptions that refer to columns), plus an undo history.
//!
//! Lengths in the graph model are in points (1 pt = 1/72 inch) unless a field
//! name says otherwise.

pub mod analysis;
pub mod analysis_link;
pub mod autorange;
pub mod clipboard;
pub mod color;
pub mod column;
pub mod datetime;
pub mod fitting;
pub mod format;
pub mod formula;
pub mod graph;
pub mod graph_style;
pub mod history;
pub mod ids;
pub mod localtime;
pub mod page_setup;
pub mod panels;
pub mod project;
pub mod reshape;
pub mod rowfilter;
pub mod scales;
pub mod shape;
pub mod sheetops;
pub mod stack;
pub mod template;
pub mod worksheet;

pub use clipboard::{CopyOptions, PasteReport};
pub use color::Color;
pub use column::{Column, ColumnData, Designation, FormulaState, RecalcMode};
pub use graph::{Axis, Graph, Layer, Plot};
pub use history::Document;
pub use ids::{ColumnId, WindowId};
pub use project::{Project, Window};
pub use worksheet::{Frozen, MAX_FROZEN, Workbook, Worksheet};
