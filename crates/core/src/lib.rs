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
pub mod batch_plot;
pub mod cellformat;
pub mod clipboard;
pub mod color;
pub mod column;
pub mod compplot;
pub mod condformat;
pub mod copy_columns;
pub mod data_range;
pub mod datetime;
pub mod extract;
pub mod fill_pattern;
pub mod fit_page;
pub mod fitting;
pub mod format;
pub mod format_clip;
pub mod formula;
pub mod graph;
pub mod graph_data;
pub mod graph_style;
pub mod history;
pub mod ids;
pub mod indexed;
pub mod layer_manage;
pub mod layer_scale;
pub mod layers;
pub mod line_series;
pub mod localtime;
pub mod marginal;
pub mod merge;
pub mod multi_y;
pub mod notes;
pub mod page_setup;
pub mod panels;
pub mod project;
pub mod qc;
pub mod reimport;
pub mod reshape;
pub mod rowfilter;
pub mod sampling;
pub mod scales;
pub mod scatter_matrix;
pub mod shape;
pub mod sheet_fonts;
pub mod sheet_print;
pub mod sheet_tab;
pub mod sheet_view;
pub mod sheetops;
pub mod stack;
pub mod stock;
pub mod straight_line;
pub mod template;
pub mod user_template;
pub mod vector;
pub mod windrose;
pub mod without_data;
pub mod worksheet;
pub mod zoom;

pub use clipboard::{CopyOptions, PasteReport};
pub use color::Color;
pub use column::{Column, ColumnData, Designation, FormulaState, RecalcMode};
pub use extract::WindowExtract;
pub use graph::{Axis, Graph, Layer, Plot};
pub use graph_data::{DataCell, DataEditError, DataEditReport};
pub use history::Document;
pub use ids::{ColumnId, WindowId};
pub use project::{Project, Window};
pub use user_template::{UserTemplate, UserTemplateError};
pub use worksheet::{Frozen, MAX_FROZEN, Workbook, Worksheet};
