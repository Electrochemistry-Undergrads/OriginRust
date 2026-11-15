//! The worksheet window as in Origin: a grey column-letter row ("A(X1)"), the light yellow label
//! rows (Long Name, Units, Comments, F(x)= and one row per user parameter) with grey row titles,
//! a grey row header, a virtualised body that lays out only the rows currently visible (so the
//! cost of drawing does not depend on how many rows a sheet has), and sheet tabs with a
//! horizontal scroll bar at the bottom. It also owns what is selected: columns, rows, a block of
//! cells or everything.

use crate::actions::{Action, CellOp, LabelRow, SheetOp};
use crate::menus::{self, MenuContext, WindowKind};
use eframe::egui::{self, Align, Color32, Layout, Pos2, Rect, RichText, Sense, Shape, Stroke, UiBuilder, vec2};
use egui_extras::{Column as TableColumn, TableBuilder};
use sheaf_core::{Column, ColumnId, Document, FormulaState, RecalcMode, WindowId, Worksheet};
use std::ops::Range;

/// What is selected in a worksheet.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum Sel {
    #[default]
    None,
    /// Whole columns; the ids are in `BookUi::selected`.
    Columns,
    /// Whole rows, from the one clicked first to the one clicked last (0-based, either order).
    Rows { anchor: usize, end: usize },
    /// A block of cells; each corner is (column position, row), 0-based.
    Cells { anchor: (usize, usize), end: (usize, usize) },
    /// One metadata cell above the data; it never targets the values underneath it.
    Label { column: ColumnId, row: LabelRow },
    /// The whole sheet.
    All,
}

/// What Copy, Cut, Clear and Paste work on, taken from the selection.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Target {
    /// Column positions in sheet order.
    pub columns: Vec<usize>,
    pub rows: Range<usize>,
    /// Where a paste starts: column position and row.
    pub top_left: (usize, usize),
}

/// The numbers in the selected cells: what the status bar summarises.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Stats {
    pub sum: f64,
    pub count: usize,
}

impl Stats {
    pub fn average(&self) -> f64 {
        self.sum / self.count as f64
    }
}

/// Sum and count of the numbers in the cells a selection covers; `None` when there is no number
/// (text and empty cells are not counted, as in Origin).
pub fn selection_stats(sheet: &Worksheet, t: &Target) -> Option<Stats> {
    let (mut sum, mut count) = (0.0, 0);
    for column in t.columns.iter().filter_map(|i| sheet.columns.get(*i)) {
        let Some(values) = column.numeric() else { continue };
        let end = t.rows.end.min(values.len());
        let start = t.rows.start.min(end);
        for v in values[start..end].iter().filter(|v| v.is_finite()) {
            sum += v;
            count += 1;
        }
    }
    (count > 0).then_some(Stats { sum, count })
}

/// A selection of columns by dragging over their heads: where it started, whether it adds to the
/// columns selected before (Ctrl), those columns, and the last head the pointer was over.
#[derive(Clone, Debug)]
struct HeadDrag {
    anchor: usize,
    additive: bool,
    base: Vec<ColumnId>,
    last: usize,
}

/// What the user is doing in one workbook window.
#[derive(Default)]
pub struct BookUi {
    /// The columns the selection touches (the whole columns for a column selection, the columns
    /// of the block for a block of cells), in the order they were clicked. Plot, Set Column
    /// Values and the analysis tools act on these.
    pub selected: Vec<ColumnId>,
    pub sel: Sel,
    /// The cell being edited: column, row, and the text typed so far.
    pub editing: Option<(ColumnId, usize, String)>,
    /// The text the editor opened with; leaving the cell with this text unchanged stores nothing.
    editing_original: String,
    /// True right after editing started, to move keyboard focus into the cell once.
    focus_editor: bool,
    /// A label cell being edited: column, which label, and the text typed so far.
    pub editing_label: Option<(ColumnId, LabelRow, String)>,
    editing_label_original: String,
    focus_label: bool,
    /// True while the mouse button is down after a press on a cell.
    dragging: bool,
    /// A drag that started on a column head.
    head_drag: Option<HeadDrag>,
    /// True while a drag that started on a row number is going on.
    row_drag: bool,
    /// The active cell inside a block of several cells (where typing starts and Enter and Tab
    /// move within the block); `None` means the cell where the block started.
    active: Option<(usize, usize)>,
    /// While a drag is going on: the pointer position pulled into the cell area, so that
    /// dragging past the edge keeps extending the selection into the cells at the edge.
    drag_pointer: Option<Pos2>,
    /// Vertical scroll position of the table, from the last frame, and a position to set.
    v_offset: f32,
    v_set: Option<f32>,
    /// Frames left in which the moving corner of the selection is scrolled into view.
    reveal: u8,
    /// Where the moving corner of the selection was drawn in the last frame.
    reveal_rect: Option<Rect>,
    /// How many rows fit in the view (for Page Up and Page Down).
    page_rows: usize,
    /// Destination to reveal while Go To leaves a row/column/range selection in place.
    navigation_target: Option<(usize, usize)>,
    /// The grid keeps the keyboard (after a click in it or editing a cell) unless a text field
    /// takes it.
    focus_grid: bool,
    /// The focus target of the grid, from the last frame.
    grid: Option<egui::Id>,
    /// The short name of a column whose header menu opens by itself in the next frame (for
    /// pictures made with `--show column-menu:NAME`).
    pub open_header_menu: Option<String>,
    /// Horizontal scroll position and the sizes it ranges over, from the last frame; the bar
    /// next to the sheet tabs draws and changes them.
    h_offset: f32,
    h_content: f32,
    h_view: f32,
}

impl BookUi {
    /// Forgets the selection (a different sheet was shown, a project was loaded).
    pub fn clear_selection(&mut self) {
        self.navigation_target = None;
        self.selected.clear();
        self.sel = Sel::None;
        self.dragging = false;
        self.head_drag = None;
        self.row_drag = false;
        self.active = None;
        self.editing = None;
        self.editing_label = None;
    }

    /// The cells the clipboard commands work on, or `None` when nothing is selected.
    pub fn target(&self, sheet: &Worksheet) -> Option<Target> {
        let all: Vec<usize> = (0..sheet.columns.len()).collect();
        match &self.sel {
            Sel::None | Sel::Label { .. } => None,
            Sel::Columns => {
                let mut columns: Vec<usize> = self.selected.iter().filter_map(|id| sheet.column_index(*id)).collect();
                columns.sort_unstable();
                columns.dedup();
                let first = *columns.first()?;
                let rows = columns.iter().filter_map(|i| sheet.columns.get(*i)).map(Column::len).max().unwrap_or(0);
                Some(Target { columns, rows: 0..rows, top_left: (first, 0) })
            }
            Sel::Rows { anchor, end } => {
                let (lo, hi) = (*anchor.min(end), *anchor.max(end));
                Some(Target { columns: all, rows: lo..hi + 1, top_left: (0, lo) })
            }
            Sel::Cells { anchor, end } => {
                let last = sheet.columns.len().checked_sub(1)?;
                let (c0, c1) = (anchor.0.min(end.0).min(last), anchor.0.max(end.0).min(last));
                let (r0, r1) = (anchor.1.min(end.1), anchor.1.max(end.1));
                Some(Target { columns: (c0..=c1).collect(), rows: r0..r1 + 1, top_left: (c0, r0) })
            }
            Sel::All => {
                let rows = sheet.row_count();
                Some(Target { columns: all, rows: 0..rows, top_left: (0, 0) })
            }
        }
    }

    /// Edit > Go to: selects one cell and scrolls it into view.
    pub fn go_to(&mut self, sheet: &Worksheet, column: usize, row: usize) {
        let to = visible_cell(sheet, (column, row), (column, row), row.max(sheet.row_count() + SPARE_ROWS - 1));
        self.select_cells(sheet, to, to);
        self.reveal = 4;
        self.focus_grid = true;
    }

    /// Origin's Go To keeps an omitted axis and extends from the opposite edge when moving
    /// before a region, otherwise from its first edge (including a target inside the region).
    pub fn go_to_selection(&mut self, sheet: &Worksheet, column: Option<usize>, row: Option<usize>, extend: bool) {
        if (column.is_none() && row.is_none()) || sheet.columns.is_empty() {
            return;
        }
        let column = column.map(|c| c.min(sheet.columns.len() - 1));
        let row = row.map(|r| r.min(self.row_capacity(sheet) - 1));
        let old = self.target(sheet);
        let cols = old.as_ref().and_then(|t| Some((*t.columns.first()?, *t.columns.last()?))).unwrap_or((0, 0));
        let rows = old.as_ref().map_or((0, 0), |t| (t.rows.start, t.rows.end.saturating_sub(1).max(t.rows.start)));
        let axis = |(lo, hi): (usize, usize), destination: Option<usize>| match destination {
            Some(to) if extend => (if to < lo { hi } else { lo }, to),
            Some(to) => (to, to),
            None => (lo, hi),
        };
        let (c0, c1) = axis(cols, column);
        let (r0, r1) = axis(rows, row);
        if row.is_none() && matches!(self.sel, Sel::Columns) {
            self.selected = sheet.columns[c0.min(c1)..=c0.max(c1)].iter().map(|c| c.id).collect();
            self.active = None;
        } else {
            self.select_cells(sheet, (c0, r0), (c1, r1));
        }
        let destination = (column.unwrap_or(c1), row.unwrap_or(r1));
        self.navigation_target = Some(destination);
        self.reveal_rect = None;
        self.reveal = 4;
        self.focus_grid = true;
    }

    pub fn row_capacity(&self, sheet: &Worksheet) -> usize {
        sheet.row_count().saturating_add(SPARE_ROWS)
    }

    /// Selects a block of cells and records which columns it touches.
    pub fn select_cells(&mut self, sheet: &Worksheet, anchor: (usize, usize), end: (usize, usize)) {
        self.navigation_target = None;
        self.sel = Sel::Cells { anchor, end };
        self.active = None;
        let (lo, hi) = (anchor.0.min(end.0), anchor.0.max(end.0));
        self.selected = sheet.columns.iter().skip(lo).take(hi + 1 - lo).map(|c| c.id).collect();
    }

    fn select_label(&mut self, column: ColumnId, row: LabelRow) {
        self.sel = Sel::Label { column, row };
        self.selected = vec![column];
        self.active = None;
        self.dragging = false;
        self.head_drag = None;
        self.row_drag = false;
    }

    fn focus(&mut self, ctx: &egui::Context) {
        self.navigation_target = None;
        self.focus_grid = true;
        if let Some(grid) = self.grid {
            ctx.memory_mut(|m| m.request_focus(grid));
        }
    }

    /// The corner of the selection that moves with Shift+arrow keys: (column position, row).
    fn active_corner(&self) -> Option<(usize, usize)> {
        match &self.sel {
            Sel::Cells { end, .. } => Some(*end),
            _ => None,
        }
    }

    /// The cell that typing edits: inside a block the active cell, else the cell where the
    /// selection started.
    pub fn active_cell(&self) -> Option<(usize, usize)> {
        match &self.sel {
            Sel::Cells { anchor, end } => {
                let (c0, c1) = (anchor.0.min(end.0), anchor.0.max(end.0));
                let (r0, r1) = (anchor.1.min(end.1), anchor.1.max(end.1));
                match self.active {
                    Some((c, r)) if (c0..=c1).contains(&c) && (r0..=r1).contains(&r) => Some((c, r)),
                    _ => Some(*anchor),
                }
            }
            _ => None,
        }
    }

    /// The cell the view follows after keys: the active cell of a block, else the moving corner.
    fn reveal_target(&self) -> Option<(usize, usize)> {
        if self.reveal > 0 && self.navigation_target.is_some() {
            return self.navigation_target;
        }
        match &self.sel {
            Sel::Cells { anchor, end } if anchor != end && self.active.is_some() => self.active_cell(),
            _ => self.active_corner(),
        }
    }

    /// The pointer position used by drags: pulled into the cell area while a drag is going on.
    fn pointer(&self, ui: &egui::Ui) -> Option<Pos2> {
        self.drag_pointer.or_else(|| ui.input(|i| i.pointer.hover_pos()))
    }

    fn select_rows(&mut self, anchor: usize, end: usize) {
        self.navigation_target = None;
        self.sel = Sel::Rows { anchor, end };
        self.selected.clear();
    }

    fn select_all(&mut self, sheet: &Worksheet) {
        self.sel = Sel::All;
        self.selected = sheet.columns.iter().map(|c| c.id).collect();
    }

    /// Drops what no longer exists (after undo, deleting columns, ...).
    fn repair(&mut self, sheet: &Worksheet) {
        self.selected.retain(|id| sheet.column(*id).is_some());
        match self.sel.clone() {
            Sel::Label { column, row } if sheet.column(column).is_none() || !label_rows(sheet).contains(&row) => {
                self.clear_selection();
            }
            Sel::Columns if self.selected.is_empty() => self.sel = Sel::None,
            Sel::Cells { anchor, end } => {
                let last = sheet.columns.len().saturating_sub(1);
                if sheet.columns.is_empty() {
                    self.clear_selection();
                } else if anchor.0 > last || end.0 > last {
                    self.select_cells(sheet, (anchor.0.min(last), anchor.1), (end.0.min(last), end.1));
                }
            }
            Sel::All => {
                if sheet.columns.is_empty() {
                    self.sel = Sel::None;
                } else {
                    self.selected = sheet.columns.iter().map(|c| c.id).collect();
                }
            }
            _ => {}
        }
        if let Some((id, ..)) = &self.editing
            && sheet.column(*id).is_none()
        {
            self.editing = None;
        }
        if let Some((id, row, _)) = &self.editing
            && (!sheet.row_shown(*row) || sheet.column(*id).is_none_or(|c| c.hidden))
        {
            self.editing = None;
        }
        if let Some((c, r)) = self.active_cell()
            && (!sheet.row_shown(r) || sheet.columns.get(c).is_none_or(|c| c.hidden))
            && let Sel::Cells { anchor, end } = self.sel
        {
            let col =
                (anchor.0.min(end.0)..=anchor.0.max(end.0)).find(|c| sheet.columns.get(*c).is_some_and(|c| !c.hidden));
            let row = (anchor.1.min(end.1)..=anchor.1.max(end.1)).find(|r| sheet.row_shown(*r));
            if let (Some(c), Some(r)) = (col, row) {
                self.active = Some((c, r));
            } else {
                let to = visible_cell(sheet, anchor, anchor, sheet.row_count() + SPARE_ROWS - 1);
                self.select_cells(sheet, to, to);
            }
            self.reveal = 4;
        }
        if let Some((id, ..)) = &self.editing_label
            && sheet.column(*id).is_none()
        {
            self.editing_label = None;
        }
    }
}

/// What the menus of the worksheet need to know: the book and what is selected right now.
fn book_ctx(book: WindowId, state: &BookUi) -> MenuContext<'_> {
    MenuContext {
        kind: WindowKind::Workbook,
        book: Some(book),
        graph: None,
        active: Some(book),
        selected: &state.selected,
        sorting_columns: if matches!(state.sel, Sel::Label { .. }) { Vec::new() } else { state.selected.clone() },
        cells: state.sel != Sel::None,
        undo: None,
        redo: None,
        windows: Vec::new(),
        open: None,
        panels: [true; 4],
        layer: 0,
        tool: Default::default(),
        curve: None,
        layer_curves: Vec::new(),
    }
}

// ----- measures and colours -------------------------------------------------------------

/// Origin's default row height and column width (about 16 px and 67 px in the pictures).
const ROW_HEIGHT: f32 = 16.0;
const COLUMN_WIDTH: f32 = 67.0;

/// The width in pixels of a column that is `chars` characters wide (8 characters are 67 px).
fn column_pixels(chars: u16) -> f32 {
    (f32::from(chars) * COLUMN_WIDTH / f32::from(sheaf_core::column::DEFAULT_WIDTH)).max(8.0)
}
/// Width of the row header together with the titles of the label rows.
const ROW_HEADER_WIDTH: f32 = 84.0;
/// Empty rows shown below the data so that new values can be typed in.
const SPARE_ROWS: usize = 30;
const TAB_BAR_HEIGHT: f32 = 20.0;

const HEADER_GREY: Color32 = Color32::from_rgb(222, 222, 222);
const LABEL_YELLOW: Color32 = Color32::from_rgb(255, 255, 214);
const ROW_SELECTED: Color32 = Color32::from_rgb(176, 196, 222);
const BLOCK_BLACK: Color32 = Color32::from_rgb(16, 16, 24);
const HEADER_DARK: Color32 = Color32::from_rgb(60, 60, 72);
const LINE_HEADER: Color32 = Color32::from_rgb(120, 120, 120);
const LINE_BODY: Color32 = Color32::from_rgb(200, 200, 200);
const TEXT: Color32 = Color32::BLACK;
/// A masked cell: light red fill and red text (Origin draws masked points red).
const MASK_FILL: Color32 = Color32::from_rgb(255, 224, 224);
const MASK_TEXT: Color32 = Color32::from_rgb(190, 0, 0);

/// The label rows of a sheet in order: the four fixed ones, then one per user parameter.
fn label_rows(sheet: &Worksheet) -> Vec<LabelRow> {
    let mut rows = vec![LabelRow::LongName, LabelRow::Units, LabelRow::Comments, LabelRow::Formula];
    rows.extend(sheet.parameter_names().into_iter().map(LabelRow::Param));
    rows
}

fn row_title(row: &LabelRow) -> String {
    match row {
        LabelRow::LongName => "Long Name".to_string(),
        LabelRow::Units => "Units".to_string(),
        LabelRow::Comments => "Comments".to_string(),
        LabelRow::Formula => "F(x)=".to_string(),
        LabelRow::Param(name) => name.clone(),
    }
}

fn label_text(column: &Column, row: &LabelRow) -> String {
    match row {
        LabelRow::LongName => column.long_name.clone(),
        LabelRow::Units => column.units.clone(),
        LabelRow::Comments => column.comments.lines().next().unwrap_or_default().to_string(),
        LabelRow::Formula => column.formula.clone().unwrap_or_default(),
        LabelRow::Param(name) => column.params.get(name).cloned().unwrap_or_default(),
    }
}

/// The name in the grey row of the head: the letter and, in brackets, the kind of data and the
/// number of the data set it belongs to ("A(X1)", "B(Y1)", "C(Y1)", "D(X2)").
pub fn head_name(sheet: &Worksheet, position: usize) -> String {
    use sheaf_core::Designation;
    let column = &sheet.columns[position];
    let tag = column.designation.tag();
    if tag.is_empty() {
        return column.short_name.clone();
    }
    match column.designation {
        Designation::Label => format!("{}({tag})", column.short_name),
        _ => {
            // The data set number counts the X columns up to this one; columns before the first
            // X belong to set 1.
            let group = sheet.columns[..=position].iter().filter(|c| c.designation == Designation::X).count().max(1);
            format!("{}({tag}{group})", column.short_name)
        }
    }
}

// ----- the window --------------------------------------------------------------------------

pub fn show(ui: &mut egui::Ui, doc: &Document, book_id: WindowId, state: &mut BookUi, actions: &mut Vec<Action>) {
    let Some(book) = doc.project().workbook(book_id) else {
        ui.label("This workbook no longer exists.");
        return;
    };
    let sheet_index = book.active_sheet.min(book.sheets.len().saturating_sub(1));
    let Some(sheet) = book.sheets.get(sheet_index) else {
        ui.label("This workbook has no sheets.");
        return;
    };
    state.repair(sheet);

    // The strip of sheet tabs along the bottom edge, the grid above it.
    let full = ui.max_rect();
    let tab_strip = Rect::from_min_max(Pos2::new(full.left(), full.bottom() - TAB_BAR_HEIGHT), full.max);
    let body = Rect::from_min_max(full.min, Pos2::new(full.right(), tab_strip.top()));
    ui.painter().rect_filled(tab_strip, 0.0, HEADER_GREY);
    ui.scope_builder(UiBuilder::new().max_rect(tab_strip), |ui| {
        sheet_tabs(ui, book, sheet_index, book_id, state, actions);
    });

    let rows = sheet.row_count() + SPARE_ROWS;
    // With an active filter the grid shows only the rows that pass, with their own numbers.
    let shown: Option<Vec<usize>> = sheet.shown_rows(rows);
    let rows_drawn = shown.as_ref().map_or(rows, Vec::len);
    let label_rows = label_rows(sheet);
    let header_height = ROW_HEIGHT * (1 + label_rows.len()) as f32;
    // The primary button was released: a drag over cells is over.
    if !ui.input(|i| i.pointer.primary_down()) {
        state.dragging = false;
        state.head_drag = None;
        state.row_drag = false;
    }
    // While dragging, the pointer counts as being in the cell area even when it is outside.
    let cells_area = Rect::from_min_max(
        Pos2::new(body.left() + 1.0, body.top() + header_height + 1.0),
        Pos2::new(body.right() - 1.0, body.bottom() - 1.0),
    );
    let drag_on = state.dragging || state.head_drag.is_some() || state.row_drag;
    state.drag_pointer =
        if drag_on { ui.input(|i| i.pointer.latest_pos()).map(|p| clamp_to(p, cells_area)) } else { None };
    state.page_rows = (((body.height() - header_height) / ROW_HEIGHT).floor() as usize).max(1);
    // The keyboard belongs to the grid once something in it was clicked: Tab and the arrow keys
    // stay with it instead of moving the focus to other widgets.
    let grid = grid_id(book_id);
    ui.interact(body, grid, Sense::focusable_noninteractive());
    ui.memory_mut(|m| {
        m.set_focus_lock_filter(
            grid,
            egui::EventFilter { tab: true, horizontal_arrows: true, vertical_arrows: true, escape: false },
        )
    });
    state.reveal_rect = None;
    state.grid = Some(grid);
    if state.focus_grid && state.editing.is_none() && state.editing_label.is_none() {
        match ui.memory(|m| m.focused()) {
            None => ui.memory_mut(|m| m.request_focus(grid)),
            Some(id) if id != grid => state.focus_grid = false,
            Some(_) => {}
        }
    }

    let output = ui
        .scope_builder(UiBuilder::new().max_rect(body), |ui| {
            egui::ScrollArea::horizontal()
                .auto_shrink([false, false])
                .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
                .horizontal_scroll_offset(state.h_offset)
                .show(ui, |ui| {
                    // Cells touch each other so the grid lines join up.
                    ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
                    ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Truncate);
                    let mut table = TableBuilder::new(ui)
                        .striped(false)
                        .resizable(true)
                        .cell_layout(Layout::left_to_right(Align::Center))
                        .column(TableColumn::exact(ROW_HEADER_WIDTH))
                        .min_scrolled_height(0.0);
                    for column in sheet.columns.iter().filter(|c| !c.hidden) {
                        table = match column.width {
                            Some(chars) => table.column(TableColumn::exact(column_pixels(chars)).clip(true)),
                            None => table.column(TableColumn::initial(COLUMN_WIDTH).at_least(30.0).clip(true)),
                        };
                    }
                    // One empty column at the end as a target for "add column".
                    table = table.column(TableColumn::remainder().at_least(40.0));
                    if state.reveal > 0
                        && let Some(row) = state.reveal_target().map(|c| c.1)
                    {
                        let at = shown.as_ref().map_or(Some(row), |s| s.iter().position(|r| *r == row));
                        if let Some(at) = at {
                            table = table.scroll_to_row(at, None);
                        }
                    }
                    if let Some(y) = state.v_set.take() {
                        table = table.vertical_scroll_offset(y);
                    }

                    table
                        .header(header_height, |mut header| {
                            header.col(|ui| corner_and_titles(ui, sheet, &label_rows, book_id, state, actions));
                            for (position, column) in sheet.columns.iter().enumerate().filter(|(_, c)| !c.hidden) {
                                header.col(|ui| {
                                    column_head(ui, sheet, position, column, &label_rows, book_id, state, actions);
                                });
                            }
                            header.col(|ui| {
                                let rect = ui.max_rect();
                                let top = Rect::from_min_size(rect.min, vec2(rect.width(), ROW_HEIGHT));
                                ui.painter().rect_filled(top, 0.0, HEADER_GREY);
                                let response = ui
                                    .interact(top, ui.id().with("add_column"), Sense::CLICK)
                                    .on_hover_text("Add a column");
                                ui.painter().text(
                                    top.center(),
                                    egui::Align2::CENTER_CENTER,
                                    "+",
                                    egui::FontId::proportional(13.0),
                                    TEXT,
                                );
                                if response.clicked() {
                                    actions.push(Action::AddColumn(book_id));
                                }
                            });
                        })
                        .body(|body| {
                            body.rows(ROW_HEIGHT, rows_drawn, |mut row| {
                                let r = shown.as_ref().map_or(row.index(), |s| s[row.index()]);
                                row.col(|ui| row_header_cell(ui, r, book_id, state, actions));
                                for (position, column) in sheet.columns.iter().enumerate().filter(|(_, c)| !c.hidden) {
                                    row.col(|ui| body_cell(ui, sheet, position, column, r, book_id, state, actions));
                                }
                                row.col(|_ui| {});
                            });
                        })
                })
        })
        .inner;
    state.h_offset = output.state.offset.x;
    state.v_offset = output.inner.state.offset.y;
    // Dragging a selection past the edge of the view scrolls the view that way.
    if drag_on && let Some(p) = ui.input(|i| i.pointer.latest_pos()) {
        let dt = ui.input(|i| i.stable_dt).min(0.1);
        let (dx, dy) = edge_scroll(p, cells_area, dt);
        if dx != 0.0 && !state.row_drag {
            let max = (output.content_size.x - output.inner_rect.width()).max(0.0);
            state.h_offset = (state.h_offset + dx).clamp(0.0, max);
        }
        if dy != 0.0 && state.head_drag.is_none() {
            state.v_set = Some((state.v_offset + dy).max(0.0));
        }
        if dx != 0.0 || dy != 0.0 {
            ui.ctx().request_repaint();
        }
    }
    if state.reveal > 0 {
        state.reveal -= 1;
        if let Some(cell) = state.reveal_rect {
            // Bring the cell into the horizontal view; the table scrolls to the row itself.
            let view = output.inner_rect;
            if cell.left() < view.left() {
                state.h_offset = (state.h_offset - (view.left() - cell.left())).max(0.0);
            } else if cell.right() > view.right() {
                let distance = cell.right() - view.right();
                state.h_offset += if state.navigation_target.is_some() { distance } else { distance.min(view.width()) };
            }
            if view.contains_rect(cell) {
                state.reveal = 0;
            }
        }
        ui.ctx().request_repaint();
    }
    if state.reveal == 0 {
        state.navigation_target = None;
    }
    state.h_content = output.content_size.x;
    state.h_view = output.inner_rect.width();
}

/// The pointer pulled into a rectangle.
fn clamp_to(p: Pos2, area: Rect) -> Pos2 {
    Pos2::new(p.x.clamp(area.left(), area.right()), p.y.clamp(area.top(), area.bottom()))
}

/// How far the view scrolls (in points, horizontally and vertically) in a frame of `dt` seconds
/// while a selection is dragged with the pointer at `p`; non-zero only past the edge of `area`,
/// faster the further out.
fn edge_scroll(p: Pos2, area: Rect, dt: f32) -> (f32, f32) {
    let speed = |outside: f32| outside.signum() * (60.0 + outside.abs() * 10.0).min(1500.0) * dt;
    let axis = |v: f32, lo: f32, hi: f32| {
        if v < lo {
            speed(v - lo)
        } else if v > hi {
            speed(v - hi)
        } else {
            0.0
        }
    };
    (axis(p.x, area.left(), area.right()), axis(p.y, area.top(), area.bottom()))
}

/// The next cell inside a block when Enter (`dy` = 1), Tab (`dx` = 1) or their Shift versions
/// (-1) are pressed: down the column and on at the top of the next column of the block (wrapping
/// from the last column to the first), or along the row and on at the start of the next row.
fn block_next(
    cols: (usize, usize),
    rows: (usize, usize),
    cur: (usize, usize),
    dx: isize,
    dy: isize,
    sheet: Option<&Worksheet>,
) -> Option<(usize, usize)> {
    let column_shown = |c: usize| sheet.is_none_or(|s| s.columns.get(c).is_some_and(|c| !c.hidden));
    let row_shown = |r| sheet.is_none_or(|s| s.row_shown(r));
    let (c, r) = cur;
    if dy != 0 {
        let (next_row, wrapped) = cycle_visible(rows, r, dy > 0, &row_shown)?;
        let next_col = if wrapped { cycle_visible(cols, c, dy > 0, &column_shown)?.0 } else { c };
        Some((next_col, next_row))
    } else if dx != 0 {
        let (next_col, wrapped) = cycle_visible(cols, c, dx > 0, &column_shown)?;
        let next_row = if wrapped { cycle_visible(rows, r, dx > 0, &row_shown)?.0 } else { r };
        Some((next_col, next_row))
    } else {
        Some(cur)
    }
}

/// Next visible position along one dimension, plus whether it wrapped to the other edge.
fn cycle_visible(
    (lo, hi): (usize, usize),
    from: usize,
    forward: bool,
    visible: &dyn Fn(usize) -> bool,
) -> Option<(usize, bool)> {
    if forward {
        (if from < hi { (from + 1..=hi).find(|i| visible(*i)) } else { None })
            .map(|i| (i, false))
            .or_else(|| (lo..=from).find(|i| visible(*i)).map(|i| (i, true)))
    } else {
        (if from > lo { (lo..from).rev().find(|i| visible(*i)) } else { None })
            .map(|i| (i, false))
            .or_else(|| (from..=hi).rev().find(|i| visible(*i)).map(|i| (i, true)))
    }
}

impl BookUi {
    /// With a block of several cells selected and `from` inside it: moves the active cell as
    /// Enter and Tab do in Origin, keeping the block selected. Returns false when the selection
    /// is not such a block.
    fn step_in_block(&mut self, from: (usize, usize), dx: isize, dy: isize, sheet: &Worksheet) -> bool {
        let Sel::Cells { anchor, end } = self.sel else { return false };
        let (c0, c1) = (anchor.0.min(end.0), anchor.0.max(end.0));
        let (r0, r1) = (anchor.1.min(end.1), anchor.1.max(end.1));
        if anchor == end || !(c0..=c1).contains(&from.0) || !(r0..=r1).contains(&from.1) {
            return false;
        }
        let Some(next) = block_next((c0, c1), (r0, r1), from, dx, dy, Some(sheet)) else {
            return false;
        };
        self.active = Some(next);
        self.reveal = 4;
        self.focus_grid = true;
        true
    }
}

// ----- sheet tabs and the horizontal scroll bar --------------------------------------------

fn sheet_tabs(
    ui: &mut egui::Ui,
    book: &sheaf_core::Workbook,
    active: usize,
    book_id: WindowId,
    state: &mut BookUi,
    actions: &mut Vec<Action>,
) {
    ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
    let bar = ui.max_rect();
    ui.horizontal(|ui| {
        // Two small arrows step to the previous and the next sheet.
        for (forward, label) in [(false, "Previous sheet"), (true, "Next sheet")] {
            let (rect, response) = ui.allocate_exact_size(vec2(16.0, TAB_BAR_HEIGHT), Sense::CLICK);
            let enabled = if forward { active + 1 < book.sheets.len() } else { active > 0 };
            let color = if enabled { TEXT } else { Color32::from_gray(150) };
            let c = rect.center();
            let s = 3.5;
            let tri = if forward {
                vec![c + vec2(-s * 0.6, -s), c + vec2(-s * 0.6, s), c + vec2(s, 0.0)]
            } else {
                vec![c + vec2(s * 0.6, -s), c + vec2(s * 0.6, s), c + vec2(-s, 0.0)]
            };
            if response.hovered() && enabled {
                ui.painter().rect_filled(rect.shrink(1.0), 0.0, Color32::from_rgb(229, 241, 251));
            }
            ui.painter().add(Shape::convex_polygon(tri, color, Stroke::NONE));
            if response.on_hover_text(label).clicked() && enabled {
                let target = if forward { active + 1 } else { active - 1 };
                actions.push(Action::SelectSheet { book: book_id, sheet: target });
            }
        }
        for (i, sheet) in book.sheets.iter().enumerate() {
            let is_active = i == active;
            let backdrop = ui.painter().add(Shape::Noop);
            let text = if is_active { RichText::new(&sheet.name).strong() } else { RichText::new(&sheet.name) };
            ui.add_space(8.0);
            let response = ui.add(egui::Label::new(text).sense(Sense::CLICK));
            ui.add_space(12.0);
            // For screen readers (and tests): a tab is a selectable item with the sheet's name.
            response.widget_info(|| {
                egui::WidgetInfo::selected(egui::WidgetType::SelectableLabel, true, is_active, sheet.name.clone())
            });
            let rect = Rect::from_min_max(
                Pos2::new(response.rect.left() - 8.0, bar.top()),
                Pos2::new(response.rect.right() + 12.0, bar.bottom()),
            );
            // A tab with a slanted right edge, white when it is the active one.
            let slant = 6.0;
            let outline = vec![
                rect.left_top(),
                rect.right_top(),
                Pos2::new(rect.right() - slant, rect.bottom()),
                rect.left_bottom(),
            ];
            let fill = if is_active { Color32::WHITE } else { HEADER_GREY };
            ui.painter().set(backdrop, Shape::convex_polygon(outline, fill, Stroke::new(1.0_f32, LINE_HEADER)));
            if response.clicked() && !is_active {
                actions.push(Action::SelectSheet { book: book_id, sheet: i });
            }
            if response.double_clicked() {
                actions.push(Action::SelectSheet { book: book_id, sheet: i });
                actions.push(Action::Sheet { book: book_id, op: SheetOp::RenameDialog });
            }
            if response.secondary_clicked() && !is_active {
                actions.push(Action::SelectSheet { book: book_id, sheet: i });
            }
            response.context_menu(|ui| {
                menus::show_context(ui, &menus::sheet_tab_menu(), &book_ctx(book_id, state), actions)
            });
        }
        ui.add_space(6.0);
        h_scroll_bar(ui, state);
    });
}

/// The slim bar right of the sheet tabs that scrolls the grid sideways.
fn h_scroll_bar(ui: &mut egui::Ui, state: &mut BookUi) {
    let width = (ui.available_width() - 6.0).max(0.0);
    let (track, response) = ui.allocate_exact_size(vec2(width, TAB_BAR_HEIGHT), Sense::CLICK | Sense::DRAG);
    let mid = track.center().y;
    let bar = Rect::from_min_max(Pos2::new(track.left(), mid - 4.0), Pos2::new(track.right(), mid + 4.0));
    ui.painter().rect_filled(bar, 0.0, Color32::from_gray(238));
    ui.painter().rect_stroke(bar, 0.0, Stroke::new(1.0_f32, LINE_BODY), egui::StrokeKind::Inside);
    let scrollable = (state.h_content - state.h_view).max(0.0);
    if state.h_content <= 0.0 || bar.width() < 10.0 {
        return;
    }
    let ratio = (state.h_view / state.h_content).clamp(0.05, 1.0);
    let thumb_w = (bar.width() * ratio).max(14.0).min(bar.width());
    let travel = (bar.width() - thumb_w).max(1.0);
    if scrollable > 0.0
        && (response.dragged() || response.clicked())
        && let Some(p) = response.interact_pointer_pos()
    {
        let t = ((p.x - bar.left() - thumb_w / 2.0) / travel).clamp(0.0, 1.0);
        state.h_offset = t * scrollable;
    }
    let t = if scrollable > 0.0 { (state.h_offset / scrollable).clamp(0.0, 1.0) } else { 0.0 };
    let thumb = Rect::from_min_size(Pos2::new(bar.left() + t * travel, bar.top()), vec2(thumb_w, bar.height()));
    ui.painter().rect_filled(thumb, 0.0, Color32::from_gray(190));
    ui.painter().rect_stroke(thumb, 0.0, Stroke::new(1.0_f32, LINE_HEADER), egui::StrokeKind::Inside);
}

// ----- the head -------------------------------------------------------------------------------

/// The top left cell (it selects everything) and, below it, the titles of the label rows.
fn corner_and_titles(
    ui: &mut egui::Ui,
    sheet: &Worksheet,
    label_rows: &[LabelRow],
    book_id: WindowId,
    state: &mut BookUi,
    actions: &mut Vec<Action>,
) {
    let rect = ui.max_rect();
    ui.painter().rect_filled(rect, 0.0, HEADER_GREY);
    let line = Stroke::new(1.0_f32, LINE_HEADER);
    let corner = Rect::from_min_size(rect.min, vec2(rect.width(), ROW_HEIGHT));
    let response = ui.interact(corner, ui.id().with("corner"), Sense::CLICK);
    if response.clicked() {
        state.select_all(sheet);
        state.focus(ui.ctx());
    }
    response.context_menu(|ui| menus::show_context(ui, &menus::corner_menu(), &book_ctx(book_id, state), actions));
    for (i, row) in label_rows.iter().enumerate() {
        let cell = Rect::from_min_size(
            Pos2::new(rect.left(), rect.top() + ROW_HEIGHT * (i + 1) as f32),
            vec2(rect.width(), ROW_HEIGHT),
        );
        title_text(ui, cell, &row_title(row));
    }
    for i in 0..=label_rows.len() {
        let y = rect.top() + ROW_HEIGHT * (i + 1) as f32 - 0.5;
        ui.painter().hline(rect.x_range(), y, line);
    }
    ui.painter().vline(rect.right() - 0.5, rect.y_range(), line);
}

/// Right-aligned text in a grey title cell.
fn title_text(ui: &mut egui::Ui, cell: Rect, text: &str) {
    let inner = cell.shrink2(vec2(4.0, 0.0));
    ui.scope_builder(UiBuilder::new().max_rect(inner).layout(Layout::right_to_left(Align::Center)), |ui| {
        ui.label(RichText::new(text).color(TEXT));
    });
}

#[allow(clippy::too_many_arguments)]
fn column_head(
    ui: &mut egui::Ui,
    sheet: &Worksheet,
    position: usize,
    column: &Column,
    label_rows: &[LabelRow],
    book_id: WindowId,
    state: &mut BookUi,
    actions: &mut Vec<Action>,
) {
    let rect = ui.max_rect();
    let selected = state.selected.contains(&column.id) && matches!(state.sel, Sel::Columns | Sel::All);
    let line = Stroke::new(1.0_f32, LINE_HEADER);
    let text_color = if selected { Color32::WHITE } else { TEXT };

    // The grey row with the name.
    let head = Rect::from_min_size(rect.min, vec2(rect.width(), ROW_HEIGHT));
    ui.painter().rect_filled(head, 0.0, if selected { HEADER_DARK } else { HEADER_GREY });
    let head_response = ui.interact(head, ui.id().with(("col_header", column.id.0)), Sense::CLICK | Sense::DRAG);
    ui.scope_builder(
        UiBuilder::new().max_rect(head).layout(Layout::centered_and_justified(egui::Direction::LeftToRight)),
        |ui| {
            ui.label(RichText::new(head_name(sheet, position)).color(text_color));
        },
    );
    if head_response.clicked() {
        let modifiers = click_modifiers(ui);
        select_column(state, sheet, column.id, modifiers);
        state.focus(ui.ctx());
    }
    head_drag(ui, state, sheet, position, &head_response, head);
    if head_response.double_clicked() {
        actions.push(Action::ColumnProperties(column.id));
    }
    if head_response.secondary_clicked() && !state.selected.contains(&column.id) {
        select_column(state, sheet, column.id, egui::Modifiers::NONE);
    }
    // The menu is built from the selection as it is after the right click.
    if state.open_header_menu.as_deref() == Some(column.short_name.as_str()) {
        // Held open under the head for a picture.
        egui::Popup::menu(&head_response)
            .open(true)
            .at_position(egui::pos2(head.left(), 24.0))
            .show(|ui| menus::show_context(ui, &menus::column_header_menu(), &book_ctx(book_id, state), actions));
    } else {
        head_response.context_menu(|ui| {
            menus::show_context(ui, &menus::column_header_menu(), &book_ctx(book_id, state), actions)
        });
    }

    // The label rows.
    for (i, row) in label_rows.iter().enumerate() {
        let cell = Rect::from_min_size(
            Pos2::new(rect.left(), rect.top() + ROW_HEIGHT * (i + 1) as f32),
            vec2(rect.width(), ROW_HEIGHT),
        );
        label_cell(ui, cell, sheet, column, row, selected, book_id, state, actions);
    }
    for i in 0..=label_rows.len() {
        let y = rect.top() + ROW_HEIGHT * (i + 1) as f32 - 0.5;
        ui.painter().hline(rect.x_range(), y, line);
    }
    ui.painter().vline(rect.right() - 0.5, rect.y_range(), line);
}

#[allow(clippy::too_many_arguments)]
fn label_cell(
    ui: &mut egui::Ui,
    cell: Rect,
    sheet: &Worksheet,
    column: &Column,
    row: &LabelRow,
    column_selected: bool,
    book_id: WindowId,
    state: &mut BookUi,
    actions: &mut Vec<Action>,
) {
    let selected = matches!(&state.sel, Sel::Label { column: c, row: r } if *c == column.id && r == row);
    if selected {
        state.reveal_rect = Some(cell);
    }
    let fill = if column_selected { BLOCK_BLACK } else { LABEL_YELLOW };
    ui.painter().rect_filled(cell, 0.0, fill);
    if selected {
        ui.painter().rect_stroke(cell.shrink(0.5), 0.0, Stroke::new(1.5_f32, Color32::BLACK), egui::StrokeKind::Inside);
    }
    let editing_here = matches!(&state.editing_label, Some((c, r, _)) if *c == column.id && r == row);
    if editing_here {
        let step = take_edit_step(ui);
        let Some((_, _, text)) = &mut state.editing_label else {
            return;
        };
        let editor = ui.id().with(("label_editor", column.id.0, row_title(row)));
        if state.focus_label {
            fresh_editor_state(ui.ctx(), editor, text);
        }
        let response = ui.put(
            cell.shrink2(vec2(1.0, 0.0)),
            egui::TextEdit::singleline(text)
                .id(editor)
                .lock_focus(true)
                .frame(false)
                .desired_width(f32::INFINITY)
                .margin(vec2(2.0, 0.0)),
        );
        if state.focus_label {
            response.request_focus();
            state.focus_label = false;
        }
        let cancel = ui.input(|i| i.key_pressed(egui::Key::Escape));
        if cancel {
            state.editing_label = None;
        } else if response.lost_focus() || step.is_some() {
            if *text != state.editing_label_original {
                actions.push(Action::SetLabel { column: column.id, row: row.clone(), text: text.clone() });
            }
            state.editing_label = None;
        }
        if state.editing_label.is_none() {
            if let Some((dx, dy)) = step {
                move_label(state, sheet, column.id, row, dx, dy);
            }
            state.focus(ui.ctx());
        }
        return;
    }
    let text = label_text(column, row);
    let color = if column_selected { Color32::WHITE } else { TEXT };
    let response = ui.interact(cell, ui.id().with(("label", column.id.0, row_title(row))), Sense::CLICK);
    response.widget_info(|| {
        egui::WidgetInfo::selected(
            egui::WidgetType::SelectableLabel,
            true,
            selected,
            format!("{} {}", column.short_name, row_title(row)),
        )
    });
    // The formula row starts with the lock mark of a column that recalculates.
    let mut left = 3.0;
    // An analysis result that recalculates shows the same lock.
    let lock = if column.is_locked() {
        Some((
            column.formula_state,
            column.recalc,
            format!("col({}) = {}", column.short_name, column.formula.as_deref().unwrap_or_default()),
        ))
    } else {
        column.analysis.map(|m| (m.state, m.mode, "Result of an analysis".to_string()))
    };
    if *row == LabelRow::Formula
        && let Some((state, mode, what)) = lock
    {
        let mark = Rect::from_center_size(Pos2::new(cell.left() + 9.0, cell.center().y), vec2(10.0, 12.0));
        lock_mark(ui, mark, column, state, mode, &what);
        left = 17.0;
    }
    let inner = Rect::from_min_max(Pos2::new(cell.left() + left, cell.top()), cell.right_bottom());
    ui.scope_builder(UiBuilder::new().max_rect(inner).layout(Layout::left_to_right(Align::Center)), |ui| {
        ui.label(RichText::new(text).color(color));
    });
    if response.clicked() {
        state.select_label(column.id, row.clone());
        state.focus(ui.ctx());
    }
    if response.secondary_clicked() {
        state.select_label(column.id, row.clone());
        state.focus(ui.ctx());
    }
    if response.double_clicked() {
        match row {
            LabelRow::Formula => actions.push(Action::SetColumnValues(column.id)),
            _ => {
                start_editing_label(state, column, row, None);
            }
        }
    }
    response.context_menu(|ui| menus::show_context(ui, &menus::label_cell_menu(), &book_ctx(book_id, state), actions));
}

/// The text a label cell is edited with (the whole comment, not just its first line).
pub fn label_text_full(column: &Column, row: &LabelRow) -> String {
    match row {
        LabelRow::Comments => column.comments.clone(),
        _ => label_text(column, row),
    }
}

fn start_editing_label(state: &mut BookUi, column: &Column, row: &LabelRow, replacement: Option<String>) {
    let original = label_text_full(column, row);
    state.editing_label_original = original.clone();
    state.editing_label = Some((column.id, row.clone(), replacement.unwrap_or(original)));
    state.focus_label = true;
}

/// Move among the metadata cells, entering the first data row after the last label.
fn move_label(state: &mut BookUi, sheet: &Worksheet, column: ColumnId, row: &LabelRow, dx: isize, dy: isize) {
    let Some(position) = sheet.column_index(column) else { return };
    let rows = label_rows(sheet);
    let Some(r) = rows.iter().position(|r| r == row) else { return };
    let c = skip_hidden(sheet, position, position.saturating_add_signed(dx).min(sheet.columns.len() - 1));
    let r = r.saturating_add_signed(dy);
    if r >= rows.len() {
        let row = skip_filtered(sheet, 0, 0, sheet.row_count() + SPARE_ROWS - 1);
        state.select_cells(sheet, (c, row), (c, row));
        state.reveal = 4;
    } else {
        state.select_label(sheet.columns[c].id, rows[r].clone());
    }
    state.reveal = 4;
}

/// The small lock in the head of a column whose formula keeps running, drawn from simple shapes:
/// closed for Auto (the column follows its inputs), with the shackle lifted for Manual (the user
/// recalculates by hand). Green when the values are current, yellow when an input changed and
/// the user has to recalculate, red when the formula cannot run (Origin shows its lock in these
/// three colours).
fn lock_mark(ui: &mut egui::Ui, rect: Rect, column: &Column, state: FormulaState, mode: RecalcMode, what: &str) {
    let (color, hint) = match (state, mode) {
        (FormulaState::Broken, _) => (
            Color32::from_rgb(200, 40, 40),
            "The formula cannot run: a column it reads is missing or renamed, or it is circular",
        ),
        (FormulaState::Pending, _) => (
            Color32::from_rgb(190, 140, 0),
            "An input changed. Use Recalculate in the column menu to update this column",
        ),
        (_, RecalcMode::Auto) => (Color32::from_rgb(40, 150, 60), "Recalculates when its inputs change"),
        _ => (Color32::from_rgb(40, 150, 60), "Up to date; recalculate by hand after an input changes"),
    };
    let state_word = match state {
        FormulaState::Broken => "broken",
        FormulaState::Pending => "pending",
        FormulaState::Current => "current",
    };
    let mode_word = if mode == RecalcMode::Auto { "auto" } else { "manual" };
    let painter = ui.painter();
    let body = Rect::from_min_max(
        Pos2::new(rect.left() + 1.0, rect.center().y),
        Pos2::new(rect.right() - 1.0, rect.bottom() - 1.0),
    );
    painter.rect_filled(body, 0.0, color);
    let stroke = Stroke::new(1.5_f32, color);
    let (l, r) = (body.left() + 2.0, body.right() - 2.0);
    let top = rect.top() + 1.0;
    let shoulder = body.top();
    // Left leg and arch.
    painter.line_segment([Pos2::new(l, shoulder), Pos2::new(l, top + 2.0)], stroke);
    painter.line_segment([Pos2::new(l, top + 2.0), Pos2::new(l + 1.5, top)], stroke);
    painter.line_segment([Pos2::new(l + 1.5, top), Pos2::new(r - 1.5, top)], stroke);
    painter.line_segment([Pos2::new(r - 1.5, top), Pos2::new(r, top + 2.0)], stroke);
    if mode == RecalcMode::Auto {
        painter.line_segment([Pos2::new(r, top + 2.0), Pos2::new(r, shoulder)], stroke);
    } else {
        // The right leg stops short of the body: the lock is open.
        painter.line_segment([Pos2::new(r, top + 2.0), Pos2::new(r, top + 3.5)], stroke);
    }
    let response = ui.interact(rect, ui.id().with(("lock", column.id.0)), Sense::hover());
    response.widget_info(|| {
        egui::WidgetInfo::labeled(
            egui::WidgetType::Label,
            true,
            format!("Lock of column {}: {mode_word}, {state_word}", column.short_name),
        )
    });
    response.on_hover_text(format!(
        "{what}
{hint}"
    ));
}

// ----- selecting ---------------------------------------------------------------------------------

/// The modifier keys held during this frame's mouse click: the keys now, and the keys that the
/// click event itself carries (a test harness, or a system that reports them only there).
fn click_modifiers(ui: &egui::Ui) -> egui::Modifiers {
    ui.input(|i| {
        let mut m = i.modifiers;
        for event in &i.events {
            if let egui::Event::PointerButton { modifiers, .. } = event {
                m.shift |= modifiers.shift;
                m.command |= modifiers.command;
                m.ctrl |= modifiers.ctrl;
                m.alt |= modifiers.alt;
            }
        }
        m
    })
}

fn select_column(state: &mut BookUi, sheet: &Worksheet, id: ColumnId, modifiers: egui::Modifiers) {
    // Starting from something else than columns: a fresh column selection.
    if state.sel != Sel::Columns {
        state.selected.clear();
    }
    state.sel = Sel::Columns;
    if modifiers.command {
        if let Some(pos) = state.selected.iter().position(|c| *c == id) {
            state.selected.remove(pos);
        } else {
            state.selected.push(id);
        }
        if state.selected.is_empty() {
            state.sel = Sel::None;
        }
    } else if modifiers.shift && !state.selected.is_empty() {
        let last = *state.selected.last().expect("not empty");
        if let (Some(a), Some(b)) = (sheet.column_index(last), sheet.column_index(id)) {
            let (lo, hi) = if a <= b { (a, b) } else { (b, a) };
            state.selected = sheet.columns[lo..=hi].iter().map(|c| c.id).collect();
        }
    } else {
        state.selected = vec![id];
    }
}

/// Selecting columns by pressing on one head and dragging over others, as if Shift-clicked; with
/// Ctrl the dragged range is added to the columns that were selected before.
fn head_drag(
    ui: &mut egui::Ui,
    state: &mut BookUi,
    sheet: &Worksheet,
    position: usize,
    response: &egui::Response,
    head: Rect,
) {
    if response.drag_started_by(egui::PointerButton::Primary) {
        let m = click_modifiers(ui);
        let additive = m.command;
        let base = if state.sel == Sel::Columns { state.selected.clone() } else { Vec::new() };
        let anchor = match (m.shift && !additive, base.last().and_then(|id| sheet.column_index(*id))) {
            (true, Some(last)) => last,
            _ => position,
        };
        state.head_drag = Some(HeadDrag { anchor, additive, base, last: usize::MAX });
        state.dragging = false;
        state.focus_grid = true;
    }
    let Some(drag) = state.head_drag.clone() else { return };
    let over = state.pointer(ui).is_some_and(|p| p.x >= head.left() && p.x < head.right());
    if !over || drag.last == position {
        return;
    }
    let (lo, hi) = (drag.anchor.min(position), drag.anchor.max(position));
    let mut selected = if drag.additive { drag.base.clone() } else { Vec::new() };
    for id in sheet.columns[lo..=hi.min(sheet.columns.len() - 1)].iter().map(|c| c.id) {
        if !selected.contains(&id) {
            selected.push(id);
        }
    }
    state.selected = selected;
    state.sel = Sel::Columns;
    if let Some(d) = &mut state.head_drag {
        d.last = position;
    }
}

/// The id of the focus target of a workbook's grid.
pub fn grid_id(book: WindowId) -> egui::Id {
    egui::Id::new(("sheet_grid", book))
}

fn row_header_cell(ui: &mut egui::Ui, r: usize, book_id: WindowId, state: &mut BookUi, actions: &mut Vec<Action>) {
    let rect = ui.max_rect();
    let selected = match &state.sel {
        Sel::Rows { anchor, end } => (*anchor.min(end)..=*anchor.max(end)).contains(&r),
        Sel::All => true,
        _ => false,
    };
    ui.painter().rect_filled(rect, 0.0, if selected { ROW_SELECTED } else { HEADER_GREY });
    let line = Stroke::new(1.0_f32, LINE_HEADER);
    ui.painter().hline(rect.x_range(), rect.bottom() - 0.5, line);
    ui.painter().vline(rect.right() - 0.5, rect.y_range(), line);
    let response = ui.interact(rect, ui.id().with(("row_header", r)), Sense::CLICK | Sense::DRAG);
    if response.drag_started_by(egui::PointerButton::Primary) {
        state.row_drag = true;
        state.dragging = false;
        state.focus_grid = true;
        match (&state.sel, click_modifiers(ui).shift) {
            (Sel::Rows { anchor, .. }, true) => {
                let anchor = *anchor;
                state.select_rows(anchor, r);
            }
            _ => state.select_rows(r, r),
        }
    } else if state.row_drag
        && state.pointer(ui).is_some_and(|p| p.y >= rect.top() && p.y < rect.bottom())
        && let Sel::Rows { anchor, end } = state.sel
        && end != r
    {
        state.select_rows(anchor, r);
    }
    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
        ui.add_space(5.0);
        ui.label(RichText::new((r + 1).to_string()).color(TEXT));
    });
    if response.clicked() {
        let shift = click_modifiers(ui).shift;
        match (&state.sel, shift) {
            (Sel::Rows { anchor, .. }, true) => {
                let anchor = *anchor;
                state.select_rows(anchor, r);
            }
            _ => state.select_rows(r, r),
        }
        state.focus_grid = true;
    }
    if response.secondary_clicked() && !selected {
        state.select_rows(r, r);
    }
    response.context_menu(|ui| menus::show_context(ui, &menus::row_menu(), &book_ctx(book_id, state), actions));
}

// ----- the cells -----------------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq)]
enum Look {
    Plain,
    /// Part of a selected column or block: black with white text.
    Dark,
    /// The cell where a block starts (white inside the black block), or a lone selected cell.
    Anchor,
}

fn cell_look(state: &BookUi, sheet: &Worksheet, position: usize, column: &Column, row: usize) -> Look {
    match &state.sel {
        Sel::Columns | Sel::All => {
            if state.selected.contains(&column.id) && row < sheet.row_count() {
                Look::Dark
            } else {
                Look::Plain
            }
        }
        Sel::Cells { anchor, end } => {
            let (c0, c1) = (anchor.0.min(end.0), anchor.0.max(end.0));
            let (r0, r1) = (anchor.1.min(end.1), anchor.1.max(end.1));
            if (c0..=c1).contains(&position) && (r0..=r1).contains(&row) {
                if Some((position, row)) == state.active_cell() { Look::Anchor } else { Look::Dark }
            } else {
                Look::Plain
            }
        }
        _ => Look::Plain,
    }
}

#[allow(clippy::too_many_arguments)]
fn body_cell(
    ui: &mut egui::Ui,
    sheet: &Worksheet,
    position: usize,
    column: &Column,
    r: usize,
    book_id: WindowId,
    state: &mut BookUi,
    actions: &mut Vec<Action>,
) {
    let rect = ui.max_rect();
    let look = cell_look(state, sheet, position, column, r);
    let single = matches!(&state.sel, Sel::Cells { anchor, end } if anchor == end);
    match look {
        Look::Dark => {
            ui.painter().rect_filled(rect, 0.0, BLOCK_BLACK);
        }
        Look::Anchor if !single => {
            ui.painter().rect_filled(rect, 0.0, Color32::WHITE);
        }
        _ if column.mask.contains(&r) => {
            ui.painter().rect_filled(rect, 0.0, MASK_FILL);
        }
        _ => {}
    }
    let (right, bottom) = (rect.right() - 0.5, rect.bottom() - 0.5);
    let grid = Stroke::new(1.0_f32, LINE_BODY);
    ui.painter().hline(rect.x_range(), bottom, grid);
    ui.painter().vline(right, rect.y_range(), grid);
    if look == Look::Anchor {
        ui.painter().rect_stroke(rect.shrink(0.5), 0.0, Stroke::new(1.5_f32, Color32::BLACK), egui::StrokeKind::Inside);
    }

    let is_editing = matches!(&state.editing, Some((c, er, _)) if *c == column.id && *er == r);
    if is_editing {
        edit_cell(ui, state, sheet, book_id, actions);
        return;
    }
    let text = if r < column.len() { column.shown_text(r) } else { String::new() };
    let numeric = column.data.is_numeric();
    // A missing number shows as an empty cell, as in Origin.
    let is_text_cell = column.data.text_cell(r).is_some();
    let text = if numeric && !is_text_cell && text == "--" { String::new() } else { text };
    let response = ui.interact(rect, ui.id().with(("cell", column.id.0, r)), Sense::CLICK | Sense::DRAG);

    // Pressing selects the cell (Shift extends the block); dragging over other cells extends it.
    let pressed = ui.input(|i| i.pointer.primary_pressed());
    let pointer = state.pointer(ui);
    let shift = click_modifiers(ui).shift;
    if state.reveal_target() == Some((position, r)) {
        state.reveal_rect = Some(rect);
    }
    if pressed && response.hovered() {
        state.dragging = true;
        state.focus(ui.ctx());
        match (&state.sel, shift) {
            (Sel::Cells { anchor, .. }, true) => {
                let anchor = *anchor;
                state.select_cells(sheet, anchor, (position, r));
            }
            _ => state.select_cells(sheet, (position, r), (position, r)),
        }
    } else if state.dragging
        && pointer.is_some_and(|p| rect.contains(p))
        && let Sel::Cells { anchor, end } = state.sel.clone()
        && end != (position, r)
    {
        state.select_cells(sheet, anchor, (position, r));
    }
    if response.secondary_clicked() && look == Look::Plain {
        state.select_cells(sheet, (position, r), (position, r));
    }

    let color = match look {
        Look::Dark => Color32::WHITE,
        _ if column.mask.contains(&r) => MASK_TEXT,
        _ => TEXT,
    };
    let layout = if numeric && !is_text_cell {
        Layout::right_to_left(Align::Center)
    } else {
        Layout::left_to_right(Align::Center)
    };
    ui.with_layout(layout, |ui| {
        ui.add_space(4.0);
        ui.label(RichText::new(&text).color(color));
    });
    // The values of a recalculating column come from its formula.
    if response.double_clicked() && !column.is_locked() {
        start_editing(state, column, r);
    }
    response.context_menu(|ui| menus::show_context(ui, &menus::cell_menu(), &book_ctx(book_id, state), actions));
}

/// The text a cell is edited with: for numbers the shortest text that reads back as exactly the
/// same value (the grid shows a rounded version, which would lose digits), empty for a missing
/// value or a row beyond the data.
fn editor_text(column: &Column, row: usize) -> String {
    match &column.data {
        sheaf_core::ColumnData::Numeric(v) => match v.get(row) {
            Some(x) if !x.is_nan() => format!("{x:?}"),
            _ => String::new(),
        },
        sheaf_core::ColumnData::Text(v) => v.get(row).cloned().unwrap_or_default(),
        sheaf_core::ColumnData::Mixed { numbers, .. } => match (column.data.text_cell(row), numbers.get(row)) {
            (Some(t), _) => t.to_string(),
            (None, Some(x)) if !x.is_nan() => format!("{x:?}"),
            _ => String::new(),
        },
    }
}

fn start_editing(state: &mut BookUi, column: &Column, row: usize) {
    let text = editor_text(column, row);
    state.editing_original = text.clone();
    state.editing = Some((column.id, row, text));
    state.focus_editor = true;
}

/// A replacement edit already holds its first typed characters. Reusing an earlier caret
/// (often at zero after Escape/Home) would insert the next character before them. Reset only
/// when opening an editor; cursor movement and text selection remain intact while editing.
fn fresh_editor_state(ctx: &egui::Context, id: egui::Id, text: &str) {
    let mut editor = egui::text_edit::TextEditState::default();
    editor.cursor.set_char_range(Some(egui::text::CCursorRange::one(egui::text::CCursor::new(text.chars().count()))));
    editor.store(ctx, id);
}

fn take_edit_step(ui: &mut egui::Ui) -> Option<(isize, isize)> {
    ui.input_mut(|i| {
        let mut step = None;
        for (key, shift, delta) in [
            (egui::Key::Tab, true, (-1, 0)),
            (egui::Key::Tab, false, (1, 0)),
            (egui::Key::Enter, true, (0, -1)),
            (egui::Key::Enter, false, (0, 1)),
            (egui::Key::ArrowUp, false, (0, -1)),
            (egui::Key::ArrowDown, false, (0, 1)),
        ] {
            let hit = i.events.iter().any(|e| {
                matches!(e, egui::Event::Key { key: k, pressed: true, modifiers, .. }
                    if *k == key && modifiers.shift == shift && !modifiers.command && !modifiers.alt)
            });
            if hit {
                step.get_or_insert(delta);
            }
        }
        if step.is_some() {
            i.events.retain(|e| {
                !matches!(e, egui::Event::Key { key, .. }
                    if matches!(key, egui::Key::Tab | egui::Key::Enter | egui::Key::ArrowUp | egui::Key::ArrowDown))
            });
        }
        step
    })
}

fn edit_cell(ui: &mut egui::Ui, state: &mut BookUi, sheet: &Worksheet, book_id: WindowId, actions: &mut Vec<Action>) {
    // Ctrl+Enter stores the text in every cell of the selected block and keeps the block, as in
    // a spreadsheet.
    let fill = ui.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::Enter));
    // Enter, Tab and Up/Down leave the cell and move on; Left/Right stay in the text. They are
    // taken before the text field sees them.
    let step = take_edit_step(ui);
    let Some((column, row, text)) = &mut state.editing else {
        return;
    };
    let editor = ui.id().with(("cell_editor", column.0, *row));
    if state.focus_editor {
        fresh_editor_state(ui.ctx(), editor, text);
    }
    // Keep egui's Tab focus traversal out of the editor. We commit and return focus to the
    // grid ourselves; otherwise a later widget can steal that focus during the same frame.
    let response = ui.add(
        egui::TextEdit::singleline(text)
            .id(editor)
            .lock_focus(true)
            .frame(false)
            .desired_width(f32::INFINITY)
            .margin(vec2(2.0, 0.0)),
    );
    if state.focus_editor {
        response.request_focus();
        state.focus_editor = false;
    }
    let cancel = ui.input(|i| i.key_pressed(egui::Key::Escape));
    if cancel {
        state.editing = None;
    } else if fill {
        actions.push(Action::Cells { book: book_id, op: CellOp::Fill(text.clone()) });
        state.editing = None;
    } else if response.lost_focus() || step.is_some() {
        let (column, row) = (*column, *row);
        if *text != state.editing_original {
            actions.push(Action::SetCell { column, row, text: text.clone() });
        }
        state.editing = None;
        let in_block = match (step, sheet.column_index(column)) {
            (Some((dx, dy)), Some(position)) => state.step_in_block((position, row), dx, dy, sheet),
            _ => false,
        };
        if let (false, Some((dx, dy)), Some(position)) = (in_block, step, sheet.column_index(column)) {
            let last_col = sheet.columns.len().saturating_sub(1);
            let to = visible_cell(
                sheet,
                (position, row),
                (
                    position.saturating_add_signed(dx).min(last_col),
                    row.saturating_add_signed(dy).min(sheet.row_count() + SPARE_ROWS - 1),
                ),
                sheet.row_count() + SPARE_ROWS - 1,
            );
            state.select_cells(sheet, to, to);
            state.reveal = 4;
            state.focus_grid = true;
        }
    }
    if state.editing.is_none() {
        state.focus_grid = true;
        if let Some(grid) = state.grid {
            ui.memory_mut(|m| m.request_focus(grid));
        }
    }
}

// ----- the keyboard ------------------------------------------------------------------------------

/// A key that moves the active cell.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NavKey {
    Up,
    Down,
    Left,
    Right,
    Home,
    End,
    PageUp,
    PageDown,
}

impl NavKey {
    fn of(key: egui::Key) -> Option<NavKey> {
        Some(match key {
            egui::Key::ArrowUp => NavKey::Up,
            egui::Key::ArrowDown => NavKey::Down,
            egui::Key::ArrowLeft => NavKey::Left,
            egui::Key::ArrowRight => NavKey::Right,
            egui::Key::Home => NavKey::Home,
            egui::Key::End => NavKey::End,
            egui::Key::PageUp => NavKey::PageUp,
            egui::Key::PageDown => NavKey::PageDown,
            _ => return None,
        })
    }
}

/// Where a navigation key takes the active cell (column position, row). `lens` has the number of
/// values of each column, `max_row` is the last row the cursor may enter (the spare rows below
/// the data), `page` the number of rows in the view.
pub fn nav_target(
    key: NavKey,
    ctrl: bool,
    from: (usize, usize),
    lens: &[usize],
    max_row: usize,
    page: usize,
) -> (usize, usize) {
    let (col, row) = from;
    let last_col = lens.len().saturating_sub(1);
    let last_data = |c: usize| lens.get(c).copied().unwrap_or(0).saturating_sub(1);
    let (col, row) = match (key, ctrl) {
        (NavKey::Up, false) => (col, row.saturating_sub(1)),
        (NavKey::Down, false) => (col, row + 1),
        (NavKey::Left, false) => (col.saturating_sub(1), row),
        (NavKey::Right, false) => (col + 1, row),
        (NavKey::Up, true) => (col, 0),
        (NavKey::Down, true) => (col, last_data(col)),
        (NavKey::Left, true) => (0, row),
        (NavKey::Right, true) => (last_col, row),
        (NavKey::Home, false) => (0, row),
        (NavKey::End, false) => (last_col, row),
        (NavKey::Home, true) => (0, 0),
        (NavKey::End, true) => (last_col, last_data(last_col)),
        (NavKey::PageUp, _) => (col, row.saturating_sub(page)),
        (NavKey::PageDown, _) => (col, row + page),
    };
    (col.min(last_col), row.min(max_row))
}

/// The column position the cursor ends on when a key aimed at `to` and the column there is
/// hidden: the next visible column in the direction of travel, else the one before it, else the
/// column it started in.
pub fn skip_hidden(sheet: &Worksheet, from: usize, to: usize) -> usize {
    let visible = |i: usize| sheet.columns.get(i).is_some_and(|c| !c.hidden);
    if visible(to) {
        return to;
    }
    let forward = to >= from;
    let first = |forward: bool| -> Option<usize> {
        if forward { (to..sheet.columns.len()).find(|i| visible(*i)) } else { (0..=to).rev().find(|i| visible(*i)) }
    };
    first(forward).or_else(|| first(!forward)).unwrap_or(from)
}

/// Skips filter-hidden rows, favouring the direction of travel and staying within the target
/// range. Rows beyond the stored filter remain visible, including the spare entry rows.
fn skip_filtered(sheet: &Worksheet, from: usize, to: usize, max_row: usize) -> usize {
    let to = to.min(max_row);
    if sheet.row_shown(to) {
        return to;
    }
    let find = |forward| {
        if forward {
            (to..=max_row).find(|r| sheet.row_shown(*r))
        } else {
            (0..=to).rev().find(|r| sheet.row_shown(*r))
        }
    };
    find(to >= from).or_else(|| find(to < from)).unwrap_or(from)
}

fn visible_cell(sheet: &Worksheet, from: (usize, usize), to: (usize, usize), max_row: usize) -> (usize, usize) {
    (skip_hidden(sheet, from.0, to.0), skip_filtered(sheet, from.1, to.1, max_row))
}

/// The keys Sheaf takes from spreadsheet programs such as Excel (none of them is bound in Origin's
/// worksheet): Ctrl+A selects the sheet, Ctrl+Space the columns and Shift+Space the rows of the
/// selection, Ctrl+Page Up and Ctrl+Page Down step through the sheet tabs, Ctrl+Plus inserts and
/// Ctrl+Minus deletes at the selection, and Backspace empties the active cell and starts editing it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SheetKey {
    SelectAll,
    SelectColumns,
    SelectRows,
    PrevSheet,
    NextSheet,
    Insert,
    Delete,
    ClearAndEdit,
}

impl SheetKey {
    fn of(key: egui::Key, m: egui::Modifiers) -> Option<SheetKey> {
        use egui::Key;
        let (ctrl, shift) = (m.command, m.shift);
        Some(match key {
            Key::A if ctrl && !shift => SheetKey::SelectAll,
            Key::Space if ctrl && !shift => SheetKey::SelectColumns,
            Key::Space if shift && !ctrl => SheetKey::SelectRows,
            Key::PageUp if ctrl && !shift => SheetKey::PrevSheet,
            Key::PageDown if ctrl && !shift => SheetKey::NextSheet,
            // Ctrl++ is Ctrl+Shift+= on most keyboards; the number pad has a key of its own.
            Key::Plus if ctrl => SheetKey::Insert,
            Key::Equals if ctrl && shift => SheetKey::Insert,
            Key::Minus if ctrl && !shift => SheetKey::Delete,
            Key::Backspace if !ctrl && !shift => SheetKey::ClearAndEdit,
            _ => return None,
        })
    }
}

/// The span of columns (positions) and rows a selection covers, for Ctrl+Space and Shift+Space.
fn sel_span(state: &BookUi, sheet: &Worksheet) -> Option<((usize, usize), (usize, usize))> {
    let last_col = sheet.columns.len().checked_sub(1)?;
    let last_row = sheet.row_count().saturating_sub(1);
    Some(match &state.sel {
        Sel::None | Sel::Label { .. } => return None,
        Sel::Cells { anchor, end } => {
            ((anchor.0.min(end.0), anchor.0.max(end.0)), (anchor.1.min(end.1), anchor.1.max(end.1)))
        }
        Sel::Rows { anchor, end } => ((0, last_col), (*anchor.min(end), *anchor.max(end))),
        Sel::All => ((0, last_col), (0, last_row)),
        Sel::Columns => {
            let positions: Vec<usize> = state.selected.iter().filter_map(|id| sheet.column_index(*id)).collect();
            ((*positions.iter().min()?, *positions.iter().max()?), (0, last_row))
        }
    })
}

/// Handles the keys of the active worksheet when no text field has the keyboard: the arrows, Tab,
/// Enter, Home, End and the page keys move the active cell (with Shift they extend the block),
/// F2 and typing start editing, and the spreadsheet keys of [`SheetKey`] select, switch sheets,
/// insert, delete or empty a cell. Returns true when keys were taken.
pub fn handle_keys(
    ctx: &egui::Context,
    doc: &Document,
    book_id: WindowId,
    state: &mut BookUi,
    actions: &mut Vec<Action>,
) -> bool {
    let Some(book) = doc.project().workbook(book_id) else { return false };
    let sheet_index = book.active_sheet.min(book.sheets.len().saturating_sub(1));
    let sheet_count = book.sheets.len();
    let Some(sheet) = book.sheets.get(sheet_index) else {
        return false;
    };
    if state.editing.is_some() || state.editing_label.is_some() {
        return false;
    }
    if matches!(state.sel, Sel::Label { .. }) {
        return handle_label_keys(ctx, sheet, book_id, sheet_index, sheet_count, state, actions);
    }
    // An empty sheet has no cells to move through, but Ctrl+Page Up and Down still leave it.
    if sheet.columns.is_empty() {
        let (prev, next) = ctx.input_mut(|i| {
            (
                i.consume_key(egui::Modifiers::COMMAND, egui::Key::PageUp),
                i.consume_key(egui::Modifiers::COMMAND, egui::Key::PageDown),
            )
        });
        let to =
            if prev { sheet_index.checked_sub(1) } else { Some(sheet_index + 1).filter(|i| next && *i < sheet_count) };
        if let Some(to) = to {
            actions.push(Action::SelectSheet { book: book_id, sheet: to });
        }
        return prev || next;
    }
    let max_row = sheet.row_count() + SPARE_ROWS - 1;
    let lens: Vec<usize> = sheet.columns.iter().map(Column::len).collect();
    let events = ctx.input(|i| i.events.clone());
    let mut taken = false;
    let mut consumed: Vec<egui::Key> = Vec::new();
    let mut typed = String::new();
    let mut edit_start = false;
    let mut clear_start = false;
    // Shift+Space also arrives as a typed space, which must not start editing.
    let mut space_taken = false;
    for event in &events {
        match event {
            egui::Event::Key { key, pressed: true, modifiers, .. } => {
                if modifiers.alt {
                    continue;
                }
                let ctrl = modifiers.command;
                let shift = modifiers.shift;
                if let Some(sheet_key) = SheetKey::of(*key, *modifiers) {
                    match sheet_key {
                        SheetKey::SelectAll => state.select_all(sheet),
                        SheetKey::SelectColumns | SheetKey::SelectRows => {
                            space_taken = true;
                            let Some(((c0, c1), (r0, r1))) = sel_span(state, sheet) else {
                                consumed.push(*key);
                                taken = true;
                                continue;
                            };
                            if sheet_key == SheetKey::SelectColumns {
                                state.sel = Sel::Columns;
                                state.selected = sheet.columns[c0..=c1].iter().map(|c| c.id).collect();
                            } else {
                                state.select_rows(r0, r1);
                            }
                            state.active = None;
                        }
                        SheetKey::PrevSheet | SheetKey::NextSheet => {
                            let to = if sheet_key == SheetKey::PrevSheet {
                                sheet_index.checked_sub(1)
                            } else {
                                Some(sheet_index + 1).filter(|i| *i < sheet_count)
                            };
                            if let Some(to) = to {
                                actions.push(Action::SelectSheet { book: book_id, sheet: to });
                            }
                        }
                        SheetKey::Insert | SheetKey::Delete => {
                            if state.sel != Sel::None {
                                let op = if sheet_key == SheetKey::Insert { CellOp::Insert } else { CellOp::Delete };
                                actions.push(Action::Cells { book: book_id, op });
                            }
                        }
                        SheetKey::ClearAndEdit => clear_start = true,
                    }
                    consumed.push(*key);
                    taken = true;
                    continue;
                }
                // In a block of several cells Enter and Tab move the active cell inside it.
                if !ctrl && matches!(key, egui::Key::Tab | egui::Key::Enter) {
                    let step = if shift { -1 } else { 1 };
                    let (dx, dy) = if *key == egui::Key::Tab { (step, 0) } else { (0, step) };
                    if let Some(from) = state.active_cell()
                        && state.step_in_block(from, dx, dy, sheet)
                    {
                        consumed.push(*key);
                        taken = true;
                        continue;
                    }
                }
                // The cell the move starts from: the moving corner when extending, the active
                // cell otherwise.
                let origin = |extend: bool| -> Option<(usize, usize)> {
                    let last_col = sheet.columns.len() - 1;
                    Some(match &state.sel {
                        Sel::Cells { anchor, end } => {
                            if extend {
                                *end
                            } else {
                                *anchor
                            }
                        }
                        Sel::Columns => {
                            let c = state.selected.last().and_then(|id| sheet.column_index(*id))?;
                            (c.min(last_col), 0)
                        }
                        Sel::Rows { end, .. } => (0, *end),
                        Sel::All => (0, 0),
                        Sel::None | Sel::Label { .. } => return None,
                    })
                };
                let (target, extend): (Option<(usize, usize)>, bool) = if let Some(nav) = NavKey::of(*key) {
                    if ctrl && matches!(nav, NavKey::PageUp | NavKey::PageDown) {
                        continue;
                    }
                    let from = origin(shift);
                    match from {
                        // Nothing selected yet: the first key press selects the first cell.
                        None => (Some(visible_cell(sheet, (0, 0), (0, 0), max_row)), false),
                        Some(from) => {
                            let to = nav_target(nav, ctrl, from, &lens, max_row, state.page_rows.max(1));
                            let limit = if ctrl && matches!(nav, NavKey::Down | NavKey::End) {
                                lens.get(to.0).copied().unwrap_or(0).saturating_sub(1).min(max_row)
                            } else {
                                max_row
                            };
                            (Some(visible_cell(sheet, from, to, limit)), shift)
                        }
                    }
                } else if ctrl {
                    continue;
                } else {
                    match key {
                        egui::Key::Tab => {
                            let nav = if shift { NavKey::Left } else { NavKey::Right };
                            let from = origin(false).unwrap_or((0, 0));
                            let to = nav_target(nav, false, from, &lens, max_row, 1);
                            (Some(visible_cell(sheet, from, to, max_row)), false)
                        }
                        egui::Key::Enter => {
                            let nav = if shift { NavKey::Up } else { NavKey::Down };
                            let from = origin(false).unwrap_or((0, 0));
                            (
                                Some(visible_cell(
                                    sheet,
                                    from,
                                    nav_target(nav, false, from, &lens, max_row, 1),
                                    max_row,
                                )),
                                false,
                            )
                        }
                        egui::Key::F2 => {
                            edit_start = true;
                            consumed.push(*key);
                            taken = true;
                            continue;
                        }
                        _ => continue,
                    }
                };
                consumed.push(*key);
                taken = true;
                let Some(to) = target else { continue };
                if extend && let Sel::Cells { anchor, .. } = state.sel {
                    state.select_cells(sheet, anchor, to);
                } else {
                    state.select_cells(sheet, to, to);
                }
                state.reveal = 4;
            }
            egui::Event::Text(text) => typed.push_str(text),
            _ => {}
        }
    }
    let mut typed: String = typed.chars().filter(|c| !c.is_control()).collect();
    if space_taken {
        typed = typed.replacen(' ', "", 1);
    }
    // Typing into the active cell (of a block, the cell where it started) replaces what it holds;
    // F2 keeps it; Backspace starts from an empty cell (storing it empties the cell).
    if let Some(active) = state.active_cell()
        && (edit_start || clear_start || !typed.is_empty())
        && let Some(column) = sheet.columns.get(active.0)
    {
        if !column.is_locked() {
            start_editing(state, column, active.1);
            if !edit_start && let Some(editing) = &mut state.editing {
                editing.2 = typed.clone();
            }
            state.reveal = 4;
        }
        taken = true;
    }
    if taken {
        ctx.input_mut(|i| {
            i.events.retain(|e| match e {
                egui::Event::Key { key, .. } => !consumed.contains(key),
                egui::Event::Text(_) => typed.is_empty() && !space_taken,
                _ => true,
            })
        });
        ctx.request_repaint();
    }
    taken
}

fn handle_label_keys(
    ctx: &egui::Context,
    sheet: &Worksheet,
    book: WindowId,
    sheet_index: usize,
    sheet_count: usize,
    state: &mut BookUi,
    actions: &mut Vec<Action>,
) -> bool {
    let events = ctx.input(|i| i.events.clone());
    let mut consumed = Vec::new();
    let mut text = String::new();
    let mut edit = false;
    let mut clear = false;
    for event in &events {
        match event {
            egui::Event::Text(t) => text.push_str(t),
            egui::Event::Key { key, pressed: true, modifiers, .. } if !modifiers.alt => {
                if modifiers.command {
                    match SheetKey::of(*key, *modifiers) {
                        Some(SheetKey::SelectAll) => state.select_all(sheet),
                        Some(SheetKey::SelectColumns) => {
                            state.sel = Sel::Columns;
                            text.clear();
                        }
                        Some(SheetKey::PrevSheet | SheetKey::NextSheet) => {
                            let to = if *key == egui::Key::PageUp {
                                sheet_index.checked_sub(1)
                            } else {
                                Some(sheet_index + 1).filter(|i| *i < sheet_count)
                            };
                            if let Some(sheet) = to {
                                actions.push(Action::SelectSheet { book, sheet });
                            }
                        }
                        _ => continue,
                    }
                } else {
                    let delta = match key {
                        egui::Key::ArrowLeft => Some((-1, 0)),
                        egui::Key::ArrowRight => Some((1, 0)),
                        egui::Key::ArrowUp => Some((0, -1)),
                        egui::Key::ArrowDown => Some((0, 1)),
                        egui::Key::Tab => Some((if modifiers.shift { -1 } else { 1 }, 0)),
                        egui::Key::Enter => Some((0, if modifiers.shift { -1 } else { 1 })),
                        egui::Key::F2 => {
                            edit = true;
                            None
                        }
                        egui::Key::Backspace => {
                            clear = true;
                            None
                        }
                        _ => continue,
                    };
                    if let (Some((dx, dy)), Sel::Label { column, row }) = (delta, state.sel.clone()) {
                        move_label(state, sheet, column, &row, dx, dy);
                    }
                }
                consumed.push(*key);
            }
            _ => {}
        }
    }
    let text: String = text.chars().filter(|c| !c.is_control()).collect();
    if (edit || clear || !text.is_empty())
        && let Sel::Label { column, row } = &state.sel
        && let Some(column) = sheet.column(*column)
    {
        if *row == LabelRow::Formula {
            actions.push(Action::SetColumnValues(column.id));
        } else {
            start_editing_label(state, column, &row.clone(), if edit { None } else { Some(text.clone()) });
        }
    }
    let taken = !consumed.is_empty() || !text.is_empty();
    if taken {
        ctx.input_mut(|i| {
            i.events.retain(|e| match e {
                egui::Event::Key { key, .. } => !consumed.contains(key),
                egui::Event::Text(_) => text.is_empty(),
                _ => true,
            })
        });
        ctx.request_repaint();
    }
    taken
}

/// A small colored square, used in menus and lists to show a plot color.
pub fn color_swatch(ui: &mut egui::Ui, color: sheaf_core::Color) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(12.0, 12.0), Sense::hover());
    ui.painter().rect_filled(rect, 2.0, Color32::from_rgba_unmultiplied(color.r, color.g, color.b, color.a));
}

#[cfg(test)]
mod tests {
    use super::*;
    use sheaf_core::{ColumnData, Designation, Project};

    fn sheet() -> Worksheet {
        let mut p = Project::new();
        let b = p.add_workbook("t");
        for (d, v) in [
            (Designation::X, vec![1.0, 2.0, 3.0]),
            (Designation::Y, vec![4.0, 5.0, 6.0, 7.0]),
            (Designation::Y, vec![8.0]),
            (Designation::X, vec![9.0, 9.0]),
            (Designation::Y, vec![1.0, 2.0]),
        ] {
            p.add_column(b, 0, d, ColumnData::numeric(v)).unwrap();
        }
        p.workbook(b).unwrap().sheet().clone()
    }

    #[test]
    fn the_cursor_steps_over_hidden_columns_in_the_direction_it_travels() {
        let mut ws = sheet();
        ws.columns[1].hidden = true;
        ws.columns[2].hidden = true;
        assert_eq!(skip_hidden(&ws, 0, 1), 3, "right: the next visible column");
        assert_eq!(skip_hidden(&ws, 3, 2), 0, "left: back to the visible column before");
        assert_eq!(skip_hidden(&ws, 0, 0), 0);
        ws.columns[3].hidden = true;
        ws.columns[4].hidden = true;
        assert_eq!(skip_hidden(&ws, 0, 1), 0, "nothing visible further right: stay");
    }

    fn filtered_sheet(keep: Vec<bool>) -> Worksheet {
        let mut ws = sheet();
        ws.filter = Some(sheaf_core::rowfilter::RowFilter { condition: "fixture".into(), enabled: true, keep });
        ws
    }

    #[test]
    fn filtered_row_targets_clamp_searches_and_reach_spare_entry_rows() {
        let mut ws = filtered_sheet(vec![false, true, false, true, false]);
        assert_eq!(skip_filtered(&ws, 1, 2, 30), 3);
        assert_eq!(skip_filtered(&ws, 3, 2, 30), 1);
        assert_eq!(skip_filtered(&ws, 3, 4, 30), 5, "new rows beyond the filter stay visible");
        assert_eq!(skip_filtered(&ws, 1, 4, 4), 3, "a data-end jump stays within the data");
        assert_eq!(skip_filtered(&ws, 3, 0, 30), 1, "Home reaches the first visible row");
        ws.filter.as_mut().unwrap().enabled = false;
        assert_eq!(skip_filtered(&ws, 1, 2, 30), 2);
    }

    #[test]
    fn block_steps_skip_both_hidden_dimensions_and_wrap_in_both_orders() {
        let mut ws = filtered_sheet(vec![true, false, true, false]);
        ws.columns[1].hidden = true;
        let step = |at, dx, dy| block_next((0, 2), (0, 3), at, dx, dy, Some(&ws));
        assert_eq!(step((0, 0), 1, 0), Some((2, 0)));
        assert_eq!(step((2, 0), 1, 0), Some((0, 2)));
        assert_eq!(step((0, 0), -1, 0), Some((2, 2)));
        assert_eq!(step((0, 0), 0, 1), Some((0, 2)));
        assert_eq!(step((0, 2), 0, 1), Some((2, 0)));
        assert_eq!(step((0, 0), 0, -1), Some((2, 2)));
        assert_eq!(block_next((0, 2), (1, 1), (0, 1), 0, 1, Some(&ws)), None);
        assert_eq!(block_next((1, 1), (0, 3), (1, 2), 1, 0, Some(&ws)), None);
    }

    #[test]
    fn a_new_filter_repairs_the_active_cell_without_editing_hidden_data() {
        let ws = filtered_sheet(vec![false, false, true, true]);
        let mut state = BookUi::default();
        state.select_cells(&ws, (1, 0), (1, 0));
        state.repair(&ws);
        assert_eq!(state.active_cell(), Some((1, 2)));
        state.select_cells(&ws, (1, 0), (2, 3));
        state.editing = Some((ws.columns[1].id, 0, "unsaved".into()));
        state.repair(&ws);
        assert!(state.editing.is_none(), "an editor hidden by filtering is cancelled");
        assert_eq!(state.active_cell(), Some((1, 2)));
        assert_eq!(
            state.sel,
            Sel::Cells { anchor: (1, 0), end: (2, 3) },
            "a visible portion of a block stays selected"
        );
        let ws = filtered_sheet(vec![false; 4]);
        state.select_cells(&ws, (0, 0), (1, 3));
        state.repair(&ws);
        assert_eq!(state.active_cell(), Some((0, 4)), "an entirely hidden block moves to a spare row");
    }

    #[test]
    fn label_navigation_and_go_to_enter_a_visible_data_cell() {
        let ws = filtered_sheet(vec![false, false, true, true]);
        let mut state = BookUi::default();
        state.go_to(&ws, 1, 0);
        assert_eq!(state.active_cell(), Some((1, 2)));
        state.go_to(&ws, 1, 1000);
        assert_eq!(state.active_cell(), Some((1, 1000)), "an explicit Go to beyond current data keeps its row");
        let labels = label_rows(&ws);
        move_label(&mut state, &ws, ws.columns[1].id, labels.last().unwrap(), 0, 1);
        assert_eq!(state.active_cell(), Some((1, 2)));
    }

    #[test]
    fn the_head_name_has_the_designation_and_the_number_of_the_data_set() {
        let s = sheet();
        let heads: Vec<String> = (0..s.columns.len()).map(|i| head_name(&s, i)).collect();
        assert_eq!(heads, ["A(X1)", "B(Y1)", "C(Y1)", "D(X2)", "E(Y2)"]);
        let mut s = s;
        s.columns[2].designation = Designation::Label;
        s.columns[1].designation = Designation::Disregard;
        assert_eq!(head_name(&s, 2), "C(L)");
        assert_eq!(head_name(&s, 1), "B");
    }

    #[test]
    fn each_kind_of_selection_names_the_columns_and_rows_the_clipboard_works_on() {
        let s = sheet();
        let mut ui = BookUi::default();
        assert_eq!(ui.target(&s), None);
        // Columns, clicked in any order, are used in sheet order; their length is the data length.
        ui.sel = Sel::Columns;
        ui.selected = vec![s.columns[2].id, s.columns[1].id];
        let t = ui.target(&s).unwrap();
        assert_eq!((t.columns, t.rows, t.top_left), (vec![1, 2], 0..4, (1, 0)));
        // Rows cover every column.
        ui.select_rows(3, 1);
        let t = ui.target(&s).unwrap();
        assert_eq!((t.columns.len(), t.rows, t.top_left), (5, 1..4, (0, 1)));
        // A block, whichever corner was pressed first.
        ui.select_cells(&s, (3, 2), (1, 0));
        let t = ui.target(&s).unwrap();
        assert_eq!((t.columns, t.rows, t.top_left), (vec![1, 2, 3], 0..3, (1, 0)));
        assert_eq!(ui.selected, vec![s.columns[1].id, s.columns[2].id, s.columns[3].id]);
        // Everything.
        ui.select_all(&s);
        let t = ui.target(&s).unwrap();
        assert_eq!((t.columns.len(), t.rows, t.top_left), (5, 0..4, (0, 0)));
    }

    #[test]
    fn the_statistics_count_the_numbers_of_the_selection_and_skip_text_and_gaps() {
        let s = sheet();
        let mut ui = BookUi::default();
        assert_eq!(ui.target(&s).and_then(|t| selection_stats(&s, &t)), None);
        // Column B alone: 4, 5, 6, 7.
        ui.sel = Sel::Columns;
        ui.selected = vec![s.columns[1].id];
        let st = selection_stats(&s, &ui.target(&s).unwrap()).unwrap();
        assert_eq!((st.sum, st.count, st.average()), (22.0, 4, 5.5));
        // A block that reaches below the shorter columns counts only what exists: C has one value.
        ui.select_cells(&s, (1, 2), (2, 3));
        let st = selection_stats(&s, &ui.target(&s).unwrap()).unwrap();
        assert_eq!((st.sum, st.count), (6.0 + 7.0, 2));
        // Everything: 6 + 22 + 8 + 18 + 3.
        ui.select_all(&s);
        let st = selection_stats(&s, &ui.target(&s).unwrap()).unwrap();
        assert_eq!((st.sum, st.count), (6.0 + 22.0 + 8.0 + 18.0 + 3.0, 3 + 4 + 1 + 2 + 2));
        // A text column holds no numbers.
        let mut text = s.clone();
        text.columns[1].data = ColumnData::text(vec!["a".into(), "b".into()]);
        ui.select_cells(&text, (1, 0), (1, 1));
        assert_eq!(selection_stats(&text, &ui.target(&text).unwrap()), None);
    }

    #[test]
    fn a_selection_that_points_beyond_the_sheet_is_pulled_back() {
        let s = sheet();
        let mut ui = BookUi::default();
        ui.select_cells(&s, (1, 0), (4, 2));
        let mut smaller = s.clone();
        smaller.columns.truncate(3);
        ui.repair(&smaller);
        assert_eq!(ui.sel, Sel::Cells { anchor: (1, 0), end: (2, 2) });
        assert_eq!(ui.selected.len(), 2);
        ui.repair(&Worksheet::new("empty"));
        assert_eq!(ui.sel, Sel::None);
    }

    #[test]
    fn navigation_keys_clamp_jump_and_enter_the_spare_rows() {
        let lens = [4, 4, 2];
        let t = |k, ctrl, from| nav_target(k, ctrl, from, &lens, 33, 10);
        assert_eq!(t(NavKey::Up, false, (0, 0)), (0, 0));
        assert_eq!(t(NavKey::Left, false, (0, 5)), (0, 5));
        assert_eq!(t(NavKey::Right, false, (2, 5)), (2, 5));
        assert_eq!(t(NavKey::Down, false, (1, 3)), (1, 4));
        assert_eq!(t(NavKey::Down, false, (1, 33)), (1, 33));
        assert_eq!(t(NavKey::Down, true, (2, 0)), (2, 1));
        assert_eq!(t(NavKey::Up, true, (2, 1)), (2, 0));
        assert_eq!(t(NavKey::Right, true, (0, 2)), (2, 2));
        assert_eq!(t(NavKey::Home, false, (2, 2)), (0, 2));
        assert_eq!(t(NavKey::End, false, (0, 2)), (2, 2));
        assert_eq!(t(NavKey::Home, true, (2, 2)), (0, 0));
        assert_eq!(t(NavKey::End, true, (0, 0)), (2, 1));
        assert_eq!(t(NavKey::PageDown, false, (1, 5)), (1, 15));
        assert_eq!(t(NavKey::PageUp, false, (1, 5)), (1, 0));
    }

    #[test]
    fn enter_and_tab_walk_a_block_and_wrap_to_the_next_column_or_row() {
        let (cols, rows) = ((1, 2), (0, 1));
        assert_eq!(block_next(cols, rows, (1, 0), 0, 1, None), Some((1, 1)));
        assert_eq!(block_next(cols, rows, (1, 1), 0, 1, None), Some((2, 0)), "the top of the next column");
        assert_eq!(block_next(cols, rows, (2, 1), 0, 1, None), Some((1, 0)), "from the end back to the start");
        assert_eq!(block_next(cols, rows, (2, 0), 0, -1, None), Some((1, 1)));
        assert_eq!(block_next(cols, rows, (1, 0), 1, 0, None), Some((2, 0)));
        assert_eq!(block_next(cols, rows, (2, 0), 1, 0, None), Some((1, 1)), "the start of the next row");
        assert_eq!(block_next(cols, rows, (2, 1), 1, 0, None), Some((1, 0)));
        assert_eq!(block_next(cols, rows, (1, 1), -1, 0, None), Some((2, 0)));
    }

    #[test]
    fn dragging_past_an_edge_scrolls_that_way_and_faster_the_further_out() {
        let area = Rect::from_min_max(Pos2::new(100.0, 50.0), Pos2::new(500.0, 350.0));
        assert_eq!(edge_scroll(Pos2::new(300.0, 200.0), area, 0.016), (0.0, 0.0));
        let (dx, dy) = edge_scroll(Pos2::new(520.0, 200.0), area, 0.016);
        assert!(dx > 0.0 && dy == 0.0);
        let (far, _) = edge_scroll(Pos2::new(700.0, 200.0), area, 0.016);
        assert!(far > dx);
        let (_, up) = edge_scroll(Pos2::new(300.0, 10.0), area, 0.016);
        assert!(up < 0.0);
        let (left, down) = edge_scroll(Pos2::new(60.0, 400.0), area, 0.016);
        assert!(left < 0.0 && down > 0.0);
        assert_eq!(clamp_to(Pos2::new(900.0, -5.0), area), Pos2::new(500.0, 50.0));
    }

    #[test]
    fn stepping_in_a_block_moves_the_active_cell_and_keeps_the_block() {
        let mut state = BookUi { sel: Sel::Cells { anchor: (0, 0), end: (1, 1) }, ..Default::default() };
        let mut ws = sheet();
        assert!(state.step_in_block((0, 0), 0, 1, &ws));
        assert_eq!(state.active_cell(), Some((0, 1)));
        assert_eq!(state.sel, Sel::Cells { anchor: (0, 0), end: (1, 1) });
        // A single cell is not stepped.
        state.sel = Sel::Cells { anchor: (0, 0), end: (0, 0) };
        assert!(!state.step_in_block((0, 0), 0, 1, &ws));
        // Tab over a hidden column: from column 0 the next column 1 is hidden, so the walk goes on.
        state.sel = Sel::Cells { anchor: (0, 0), end: (2, 0) };
        ws.columns[1].hidden = true;
        assert!(state.step_in_block((0, 0), 1, 0, &ws));
        assert_eq!(state.active_cell(), Some((2, 0)));
    }
}
