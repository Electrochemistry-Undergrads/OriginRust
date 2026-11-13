//! The toolbars (ADR 0007, point 3): docked rows of small flat icon buttons.
//!
//! Origin has a Standard bar on top, an Edit and Format bar under it, a Graph bar, a 2D Graphs
//! bar at the bottom, a Tools bar down the left edge and an Object Edit bar down the right edge
//! (study: `docs/spec/ui-command-map.md`, section C, and `docs/spec/gui-look.md`). This module
//! holds the table of buttons of each bar and draws them.
//!
//! A button is an icon, a tooltip with Origin's name for the button and a [`Cmd`], the same
//! command the menu entry of that name sends: a click is resolved with [`menus::resolve`], so it
//! does exactly what the menu does, and is greyed for the same reasons (wrong window type, nothing
//! selected). A button without a command is a feature Sheaf lacks; it is drawn greyed and does
//! nothing. The one exception is the Pointer of the Tools bar, which is always the tool in use.
//!
//! The icons are our own: simple shapes drawn with egui's painter in a 16 by 16 pixel box and two
//! colours. They are recognisable by meaning (a grid for a workbook, a polyline for Line, dots
//! for Scatter) and are not copies of Origin's pictures.

use crate::actions::{Action, GraphTool};
use crate::dialogs::analysis::AnalysisKind;
use crate::menus::{self, Cmd, MenuContext, NOT_YET};
use crate::panels::PanelId;
use crate::theme;
use eframe::egui::{self, Align2, Color32, FontId, Painter, Pos2, Rect, Sense, Shape, Stroke, Ui, vec2};
use sheaf_core::Designation;
use sheaf_core::template::Template;
use std::collections::HashSet;
use std::sync::OnceLock;

/// One of the toolbars.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Bar {
    Standard,
    Edit,
    Format,
    Graph,
    Graphs2D,
    Tools,
    ObjectEdit,
}

impl Bar {
    pub const ALL: [Bar; 7] =
        [Bar::Standard, Bar::Edit, Bar::Format, Bar::Graph, Bar::Graphs2D, Bar::Tools, Bar::ObjectEdit];

    /// Origin's name of the bar (the list of View > Toolbars).
    pub fn name(self) -> &'static str {
        match self {
            Bar::Standard => "Standard",
            Bar::Edit => "Edit",
            Bar::Format => "Format",
            Bar::Graph => "Graph",
            Bar::Graphs2D => "2D Graphs",
            Bar::Tools => "Tools",
            Bar::ObjectEdit => "Object Edit",
        }
    }
}

/// Which bars are shown, and whether the window that lists them is open. Kept for the session.
#[derive(Default)]
pub struct Toolbars {
    hidden: HashSet<Bar>,
    pub dialog: bool,
}

impl Toolbars {
    pub fn shown(&self, bar: Bar) -> bool {
        !self.hidden.contains(&bar)
    }

    pub fn set(&mut self, bar: Bar, shown: bool) {
        if shown {
            self.hidden.remove(&bar);
        } else {
            self.hidden.insert(bar);
        }
    }

    /// True when at least one of the bars is shown.
    pub fn any(&self, bars: &[Bar]) -> bool {
        bars.iter().any(|b| self.shown(*b))
    }

    /// The window of View > Toolbars: a tick box for each bar.
    pub fn show_dialog(&mut self, ctx: &egui::Context) {
        let mut open = self.dialog;
        egui::Window::new("Toolbars").open(&mut open).collapsible(false).resizable(false).show(ctx, |ui| {
            for bar in Bar::ALL {
                let mut shown = self.shown(bar);
                if ui.checkbox(&mut shown, bar.name()).changed() {
                    self.set(bar, shown);
                }
            }
        });
        self.dialog = open;
    }
}

// ----- the table ---------------------------------------------------------------------------

/// What a button draws. The names say what the picture shows.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Icon {
    Project,
    Workbook,
    Graph,
    Matrix,
    Layout,
    Notes,
    OpenFolder,
    Disk,
    Recalc,
    Import,
    ImportMany,
    Print,
    Duplicate,
    PanelLeft,
    PanelRight,
    LogLines,
    AddColumns,
    Sigma,
    SortUp,
    SortDown,
    Fx,
    Filter,
    Letter(char),
    ErrorBar,
    Tag,
    Disregard,
    Cut,
    Copy,
    Paste,
    Undo,
    Redo,
    Cascade,
    TileH,
    TileV,
    // Format
    Bold,
    Italic,
    Underline,
    Superscript,
    Subscript,
    Greek,
    Bigger,
    Smaller,
    FontColor,
    FillColor,
    LineColor,
    // Graph and Tools
    Pointer,
    ZoomIn,
    ZoomOut,
    WholePage,
    Rescale,
    Exchange,
    Layer,
    LayerTop,
    LayerRight,
    LayerBoth,
    Inset,
    DataReader,
    Text,
    Arrow,
    Line,
    Rectangle,
    Hand,
    // 2D graphs
    PlotLine,
    PlotScatter,
    PlotLineSymbol,
    PlotColumn,
    PlotBar,
    PlotPie,
    PlotArea,
    PlotStackedArea,
    PlotHlc,
    PlotBox,
    PlotHistogram,
    PlotBubble,
    PlotPolar,
    PlotTernary,
    PlotRadar,
    // Object Edit
    AlignLeft,
    AlignRight,
    AlignTop,
    AlignBottom,
    AlignCenterH,
    AlignCenterV,
    SameWidth,
    SameHeight,
    Front,
    Back,
    Group,
    Ungroup,
}

pub struct Button {
    pub icon: Icon,
    /// Origin's name of the button, shown as the tooltip.
    pub tip: &'static str,
    /// The command of the menu entry the button stands for; `None` when Sheaf lacks it.
    pub cmd: Option<Cmd>,
    /// The name of that menu entry (without Alt letter and dots), for the test that compares the two.
    #[allow(dead_code)] // read by the test that compares buttons with menu entries
    pub menu: &'static str,
    /// A tool that is always switched on (the Pointer): drawn pressed, not greyed, does nothing.
    pub on: bool,
}

pub enum Item {
    Button(Button),
    /// A thin gap between groups.
    Gap,
    /// The font name box of the Format bar (Sheaf cannot apply fonts to objects yet).
    FontBox,
    /// The font size box.
    SizeBox,
}

fn b(icon: Icon, tip: &'static str, cmd: Cmd, menu: &'static str) -> Item {
    Item::Button(Button { icon, tip, cmd: Some(cmd), menu, on: false })
}

/// A tool of the Tools toolbar: it has no menu entry in Origin; it is drawn pressed while it is
/// the graph's tool.
fn tool(icon: Icon, tip: &'static str, cmd: Cmd) -> Item {
    Item::Button(Button { icon, tip, cmd: Some(cmd), menu: "", on: false })
}

/// True for the commands of tools (no menu entry; pressed while on).
#[cfg(test)]
fn is_tool(cmd: &Cmd) -> bool {
    matches!(cmd, Cmd::Pointer | Cmd::DataReader | Cmd::TextTool)
}

/// A button for a feature Sheaf lacks.
fn g(icon: Icon, tip: &'static str) -> Item {
    Item::Button(Button { icon, tip, cmd: None, menu: "", on: false })
}

pub fn items(bar: Bar) -> &'static [Item] {
    static TABLE: OnceLock<Vec<Vec<Item>>> = OnceLock::new();
    let table = TABLE.get_or_init(|| Bar::ALL.iter().map(|b| build(*b)).collect());
    &table[Bar::ALL.iter().position(|x| *x == bar).expect("a bar of the list")]
}

fn build(bar: Bar) -> Vec<Item> {
    use Icon::*;
    let gap = || Item::Gap;
    match bar {
        Bar::Standard => vec![
            b(Project, "New Project", Cmd::NewProject, "Project"),
            b(Workbook, "New Workbook", Cmd::NewWorkbook, "Worksheet"),
            g(Graph, "New Graph"),
            g(Matrix, "New Matrix"),
            g(Layout, "New Layout"),
            g(Notes, "New Notes"),
            gap(),
            b(OpenFolder, "Open", Cmd::OpenProject, "Open"),
            b(Disk, "Save Project", Cmd::SaveProject, "Save Project"),
            gap(),
            b(Recalc, "Recalculate", Cmd::Recalculate, "Recalculate"),
            gap(),
            b(Import, "Import Single ASCII", Cmd::Import, "Single ASCII"),
            b(ImportMany, "Import Multiple ASCII", Cmd::ImportMultiple, "Multiple ASCII"),
            gap(),
            g(Print, "Print"),
            g(Duplicate, "Duplicate"),
            gap(),
            b(PanelLeft, "Project Explorer", Cmd::Panel(PanelId::Explorer), "Project Explorer"),
            b(PanelRight, "Object Manager", Cmd::Panel(PanelId::Objects), "Object Manager"),
            b(LogLines, "Results Log", Cmd::Panel(PanelId::Results), "Results Log"),
            gap(),
            b(AddColumns, "Add New Columns", Cmd::AddColumns, "Add New Columns"),
            gap(),
            b(Sigma, "Statistics on Columns", Cmd::Analyze(AnalysisKind::Statistics), "Statistics on Columns"),
            b(SortUp, "Sort Worksheet Ascending", Cmd::SortAscending, "Ascending"),
            b(SortDown, "Sort Worksheet Descending", Cmd::SortDescending, "Descending"),
            b(Fx, "Set Column Values", Cmd::SetColumnValues, "Set Column Values"),
            b(Filter, "Worksheet Query", Cmd::FilterRows, "Worksheet Query"),
            gap(),
            b(Letter('X'), "Set as X", Cmd::SetAs(Designation::X), "X"),
            b(Letter('Y'), "Set as Y", Cmd::SetAs(Designation::Y), "Y"),
            b(Letter('Z'), "Set as Z", Cmd::SetAs(Designation::Z), "Z"),
            b(ErrorBar, "Set as Y Error", Cmd::SetAs(Designation::YError), "Y Error"),
            b(Tag, "Set as Label", Cmd::SetAs(Designation::Label), "Label"),
            b(Disregard, "Set as Disregard", Cmd::SetAs(Designation::Disregard), "Disregard"),
            gap(),
            b(Cascade, "Cascade", Cmd::Cascade, "Cascade"),
            b(TileH, "Tile Horizontally", Cmd::TileHorizontally, "Tile Horizontally"),
            b(TileV, "Tile Vertically", Cmd::TileVertically, "Tile Vertically"),
        ],
        Bar::Edit => vec![
            b(Cut, "Cut", Cmd::Cut, "Cut"),
            b(Icon::Copy, "Copy", Cmd::Copy(crate::actions::CopyKind::Plain), "Copy"),
            b(Paste, "Paste", Cmd::Paste, "Paste"),
            b(Undo, "Undo", Cmd::Undo, "Undo"),
            b(Redo, "Redo", Cmd::Redo, "Redo"),
        ],
        Bar::Format => vec![
            Item::FontBox,
            Item::SizeBox,
            gap(),
            g(Bold, "Bold"),
            g(Italic, "Italic"),
            g(Underline, "Underline"),
            g(Superscript, "Superscript"),
            g(Subscript, "Subscript"),
            g(Greek, "Greek"),
            g(Bigger, "Increase Font Size"),
            g(Smaller, "Decrease Font Size"),
            gap(),
            g(FontColor, "Font Color"),
            g(FillColor, "Fill Color"),
            g(LineColor, "Line Color"),
        ],
        Bar::Graph => vec![
            g(ZoomIn, "Zoom In"),
            g(ZoomOut, "Zoom Out"),
            g(WholePage, "Whole Page"),
            b(Rescale, "Rescale to Show All", Cmd::Rescale, "Rescale to Show All"),
            g(Exchange, "Exchange X-Y Axes"),
            gap(),
            g(Layer, "Add Layer (Bottom-X Left-Y)"),
            g(LayerTop, "Add Layer (Top-X Left-Y)"),
            g(LayerRight, "Add Layer (Bottom-X Right-Y)"),
            g(LayerBoth, "Add Layer (Top-X Right-Y)"),
            gap(),
            g(Inset, "Add Inset Graph"),
        ],
        Bar::Graphs2D => vec![
            b(PlotLine, "Line", Cmd::Plot(Template::Line), "Line"),
            b(PlotScatter, "Scatter", Cmd::Plot(Template::Scatter), "Scatter"),
            b(PlotLineSymbol, "Line + Symbol", Cmd::Plot(Template::LineSymbol), "Line + Symbol"),
            b(PlotColumn, "Column", Cmd::Plot(Template::Column), "Column"),
            g(PlotBar, "Bar"),
            g(PlotPie, "Pie"),
            b(PlotArea, "Area", Cmd::Plot(Template::Area), "Area"),
            g(PlotStackedArea, "Stacked Area"),
            g(PlotHlc, "High-Low-Close"),
            g(PlotBox, "Box Chart"),
            g(PlotHistogram, "Histogram"),
            g(PlotBubble, "Bubble"),
            g(PlotPolar, "Polar theta(X) r(Y)"),
            g(PlotTernary, "Ternary"),
            g(PlotRadar, "Radar"),
        ],
        Bar::Tools => vec![
            tool(Pointer, "Pointer", Cmd::Pointer),
            g(ZoomIn, "Zoom In"),
            g(ZoomOut, "Zoom Out"),
            b(Rescale, "Rescale", Cmd::Rescale, "Rescale to Show All"),
            tool(DataReader, "Data Reader", Cmd::DataReader),
            gap(),
            tool(Text, "Text", Cmd::TextTool),
            g(Arrow, "Arrow"),
            g(Icon::Line, "Line"),
            g(Rectangle, "Rectangle"),
            g(Hand, "Pan"),
        ],
        Bar::ObjectEdit => vec![
            g(AlignLeft, "Align Left"),
            g(AlignRight, "Align Right"),
            g(AlignTop, "Align Top"),
            g(AlignBottom, "Align Bottom"),
            g(AlignCenterH, "Align Horizontal Center"),
            g(AlignCenterV, "Align Vertical Center"),
            gap(),
            g(SameWidth, "Make Same Width"),
            g(SameHeight, "Make Same Height"),
            gap(),
            g(Front, "Bring to Front"),
            g(Back, "Send to Back"),
            g(Group, "Group"),
            g(Ungroup, "Ungroup"),
        ],
    }
}

// ----- drawing the bars --------------------------------------------------------------------

const CELL: f32 = 22.0;

/// A row of bars side by side, each with a grip at its left end.
pub fn show_row(ui: &mut Ui, bars: &[Bar], tb: &Toolbars, c: &MenuContext, actions: &mut Vec<Action>) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
        let mut first = true;
        for bar in bars.iter().filter(|b| tb.shown(**b)) {
            if !first {
                ui.add_space(8.0);
            }
            first = false;
            show_bar(ui, *bar, c, actions);
        }
    });
}

/// One bar in the layout the caller has set up: a row, or a column for the Tools and Object Edit
/// bars.
pub fn show_bar(ui: &mut Ui, bar: Bar, c: &MenuContext, actions: &mut Vec<Action>) {
    grip(ui);
    for item in items(bar) {
        match item {
            Item::Gap => gap(ui),
            Item::FontBox => text_box(ui, "Default: Arial", 118.0, "Font"),
            Item::SizeBox => text_box(ui, "", 44.0, "Font Size"),
            Item::Button(button) => draw_button(ui, bar, button, c, actions),
        }
    }
}

fn horizontal(ui: &Ui) -> bool {
    ui.layout().main_dir().is_horizontal()
}

/// The dotted handle at the start of a bar.
fn grip(ui: &mut Ui) {
    let size = if horizontal(ui) { vec2(7.0, CELL) } else { vec2(CELL, 7.0) };
    let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
    let dot = Color32::from_rgb(150, 152, 158);
    let painter = ui.painter();
    for i in 0..6 {
        let t = 3.0 + 3.0 * i as f32;
        let p = if size.x < size.y {
            Pos2::new(rect.center().x, rect.top() + t)
        } else {
            Pos2::new(rect.left() + t, rect.center().y)
        };
        painter.rect_filled(Rect::from_center_size(p, vec2(1.0, 1.0)), 0.0, dot);
    }
}

fn gap(ui: &mut Ui) {
    let size = if horizontal(ui) { vec2(7.0, CELL) } else { vec2(CELL, 7.0) };
    let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
    let stroke = Stroke::new(1.0_f32, Color32::from_rgb(205, 205, 210));
    if size.x < size.y {
        ui.painter().vline(rect.center().x.floor() + 0.5, rect.y_range().shrink(3.0), stroke);
    } else {
        ui.painter().hline(rect.x_range().shrink(3.0), rect.center().y.floor() + 0.5, stroke);
    }
}

/// A box like the font name and size lists, greyed: Sheaf cannot format objects yet.
fn text_box(ui: &mut Ui, text: &str, width: f32, name: &str) {
    let (rect, response) = ui.allocate_exact_size(vec2(width, 20.0), Sense::hover());
    let rect = rect.shrink2(vec2(1.0, 0.0));
    let grey = Color32::from_rgb(170, 172, 178);
    let painter = ui.painter();
    painter.rect_filled(rect, 0.0, Color32::from_rgb(244, 244, 246));
    painter.add(Shape::closed_line(
        vec![rect.left_top(), rect.right_top(), rect.right_bottom(), rect.left_bottom()],
        Stroke::new(1.0_f32, Color32::from_rgb(210, 210, 214)),
    ));
    painter.text(rect.left_center() + vec2(4.0, 0.0), Align2::LEFT_CENTER, text, FontId::proportional(11.0), grey);
    let a = Pos2::new(rect.right() - 8.0, rect.center().y);
    painter.add(Shape::convex_polygon(
        vec![a + vec2(-3.0, -1.5), a + vec2(3.0, -1.5), a + vec2(0.0, 1.5)],
        grey,
        Stroke::NONE,
    ));
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::ComboBox, false, name));
    response.on_hover_ui(|ui| {
        ui.label(name);
        ui.weak(NOT_YET);
    });
}

fn draw_button(ui: &mut Ui, bar: Bar, button: &Button, c: &MenuContext, actions: &mut Vec<Action>) {
    let resolved = button.cmd.as_ref().map(|cmd| menus::resolve(cmd, c));
    let on = match button.cmd {
        Some(Cmd::Pointer) => c.tool == GraphTool::Pointer,
        Some(Cmd::DataReader) => c.tool == GraphTool::Reader,
        Some(Cmd::TextTool) => c.tool == GraphTool::Text,
        _ => button.on,
    };
    let enabled = on || matches!(resolved, Some(Ok(_)));
    let sense = if enabled && !on { Sense::click() } else { Sense::hover() };
    let (rect, response) = ui.allocate_exact_size(vec2(CELL, CELL), sense);
    let label = format!("{} toolbar: {}", bar.name(), button.tip);
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, enabled, &label));

    let painter = ui.painter();
    let frame = if on || (enabled && response.is_pointer_button_down_on()) {
        Some((Color32::from_rgb(204, 228, 247), Color32::from_rgb(0, 84, 153)))
    } else if enabled && response.hovered() {
        Some((Color32::from_rgb(229, 241, 251), theme::ACCENT))
    } else {
        None
    };
    if let Some((fill, line)) = frame {
        let r = rect.shrink(1.0);
        painter.rect_filled(r, 0.0, fill);
        painter.add(Shape::closed_line(
            vec![r.left_top(), r.right_top(), r.right_bottom(), r.left_bottom()],
            Stroke::new(1.0_f32, line),
        ));
    }
    paint_icon(painter, Rect::from_center_size(rect.center(), vec2(16.0, 16.0)), button.icon, enabled);

    let reason = match &resolved {
        Some(Err(why)) => Some(*why),
        None if !on => Some(NOT_YET),
        _ => None,
    };
    let name = if matches!(button.cmd, Some(Cmd::Undo | Cmd::Redo)) && enabled {
        let what = if button.cmd == Some(Cmd::Undo) { c.undo } else { c.redo };
        format!("{} {}", button.tip, what.unwrap_or_default())
    } else {
        button.tip.to_string()
    };
    let clicked = enabled && response.clicked();
    if response.hovered() {
        crate::panels::set_hint(ui.ctx(), &name);
    }
    response.on_hover_ui(|ui| {
        ui.label(name);
        if let Some(why) = reason {
            ui.weak(why);
        }
    });
    if clicked && let Some(Ok(action)) = resolved {
        actions.push(action);
    }
}

// ----- the icons ---------------------------------------------------------------------------

const MAIN: Color32 = Color32::from_rgb(52, 58, 75);
const ACCENT: Color32 = Color32::from_rgb(0, 120, 215);
const MAIN_OFF: Color32 = Color32::from_rgb(168, 170, 176);
const ACCENT_OFF: Color32 = Color32::from_rgb(196, 204, 214);

/// Draws in a 16 by 16 box with the two colours (washed out when `enabled` is false).
pub fn paint_icon(painter: &Painter, rect: Rect, icon: Icon, enabled: bool) {
    let pen = Pen {
        p: painter,
        o: rect.min,
        main: if enabled { MAIN } else { MAIN_OFF },
        accent: if enabled { ACCENT } else { ACCENT_OFF },
        mirror: false,
    };
    pen.icon(icon);
}

#[derive(Clone, Copy)]
struct Pen<'a> {
    p: &'a Painter,
    o: Pos2,
    main: Color32,
    accent: Color32,
    /// Draw left to right reversed (Redo is Undo turned around).
    mirror: bool,
}

type P = (f32, f32);

impl Pen<'_> {
    fn pt(&self, (x, y): P) -> Pos2 {
        Pos2::new(self.o.x + if self.mirror { 16.0 - x } else { x }, self.o.y + y)
    }

    fn colour(&self, accent: bool) -> Color32 {
        if accent { self.accent } else { self.main }
    }

    fn pts(&self, pts: &[P]) -> Vec<Pos2> {
        pts.iter().map(|p| self.pt(*p)).collect()
    }

    fn line(&self, pts: &[P], accent: bool) {
        self.p.add(Shape::line(self.pts(pts), Stroke::new(1.2_f32, self.colour(accent))));
    }

    fn closed(&self, pts: &[P], accent: bool) {
        self.p.add(Shape::closed_line(self.pts(pts), Stroke::new(1.2_f32, self.colour(accent))));
    }

    fn rect(&self, x0: f32, y0: f32, x1: f32, y1: f32, accent: bool) {
        self.closed(&[(x0, y0), (x1, y0), (x1, y1), (x0, y1)], accent);
    }

    fn fill_rect(&self, x0: f32, y0: f32, x1: f32, y1: f32, accent: bool) {
        self.fill(&[(x0, y0), (x1, y0), (x1, y1), (x0, y1)], accent);
    }

    /// A filled convex polygon.
    fn fill(&self, pts: &[P], accent: bool) {
        self.p.add(Shape::convex_polygon(self.pts(pts), self.colour(accent), Stroke::NONE));
    }

    fn circle(&self, c: P, r: f32, accent: bool) {
        self.p.circle_stroke(self.pt(c), r, Stroke::new(1.2_f32, self.colour(accent)));
    }

    fn dot(&self, c: P, r: f32, accent: bool) {
        self.p.circle_filled(self.pt(c), r, self.colour(accent));
    }

    fn text(&self, c: P, s: &str, size: f32, accent: bool) {
        self.p.text(self.pt(c), Align2::CENTER_CENTER, s, FontId::proportional(size), self.colour(accent));
    }

    /// The two axes of a chart.
    fn axes(&self) {
        self.line(&[(2.0, 1.5), (2.0, 14.0), (15.0, 14.0)], false);
    }

    /// An arrow from one point to another with a head at the end.
    fn arrow(&self, from: P, to: P, accent: bool) {
        self.line(&[from, to], accent);
        let (dx, dy) = (to.0 - from.0, to.1 - from.1);
        let len = (dx * dx + dy * dy).sqrt().max(0.001);
        let (ux, uy) = (dx / len, dy / len);
        let back = (to.0 - ux * 4.0, to.1 - uy * 4.0);
        let (nx, ny) = (-uy * 2.5, ux * 2.5);
        self.fill(&[to, (back.0 + nx, back.1 + ny), (back.0 - nx, back.1 - ny)], accent);
    }

    /// A circular arc as a line, angles in degrees, with the arrow head at its start.
    fn arc(&self, c: P, r: f32, from: f32, to: f32, accent: bool) -> Vec<P> {
        let n = 12;
        let pts: Vec<P> = (0..=n)
            .map(|i| {
                let a = (from + (to - from) * i as f32 / n as f32).to_radians();
                (c.0 + r * a.cos(), c.1 + r * a.sin())
            })
            .collect();
        self.line(&pts, accent);
        pts
    }

    fn icon(&self, icon: Icon) {
        use Icon::*;
        match icon {
            Project => {
                self.closed(&[(3.0, 1.5), (10.0, 1.5), (13.0, 4.5), (13.0, 14.5), (3.0, 14.5)], false);
                self.line(&[(10.0, 1.5), (10.0, 4.5), (13.0, 4.5)], false);
                self.line(&[(5.5, 8.5), (10.5, 8.5)], true);
                self.line(&[(5.5, 11.5), (10.5, 11.5)], true);
            }
            Workbook => {
                self.fill_rect(2.0, 2.0, 14.0, 5.5, true);
                self.rect(2.0, 2.0, 14.0, 14.0, false);
                self.line(&[(2.0, 8.5), (14.0, 8.5)], false);
                self.line(&[(2.0, 11.5), (14.0, 11.5)], false);
                self.line(&[(6.0, 2.0), (6.0, 14.0)], false);
                self.line(&[(10.0, 2.0), (10.0, 14.0)], false);
            }
            Graph => {
                self.axes();
                self.line(&[(4.0, 11.0), (7.0, 6.5), (10.0, 9.0), (14.0, 3.0)], true);
            }
            Matrix => {
                self.rect(2.0, 2.0, 14.0, 14.0, false);
                for x in [5.5, 8.0, 10.5] {
                    for y in [5.5, 8.0, 10.5] {
                        self.dot((x, y), 1.0, true);
                    }
                }
            }
            Layout => {
                self.rect(2.0, 1.5, 14.0, 14.5, false);
                self.rect(4.0, 3.5, 12.0, 8.0, true);
                self.rect(4.0, 10.0, 7.5, 12.5, false);
                self.rect(8.5, 10.0, 12.0, 12.5, false);
            }
            Notes => {
                self.closed(&[(3.0, 1.5), (13.0, 1.5), (13.0, 14.5), (3.0, 14.5)], false);
                for y in [5.0, 8.0, 11.0] {
                    self.line(&[(5.5, y), (10.5, y)], true);
                }
            }
            OpenFolder => {
                self.line(&[(1.5, 13.5), (1.5, 3.5), (6.0, 3.5), (7.5, 5.5), (13.0, 5.5), (13.0, 8.0)], false);
                self.closed(&[(1.5, 13.5), (4.0, 8.0), (15.0, 8.0), (12.5, 13.5)], true);
            }
            Disk => {
                self.closed(&[(2.0, 2.0), (12.0, 2.0), (14.0, 4.0), (14.0, 14.0), (2.0, 14.0)], false);
                self.fill_rect(5.0, 2.0, 11.0, 6.0, true);
                self.rect(4.5, 9.0, 11.5, 14.0, false);
            }
            Recalc => {
                let pts = self.arc((8.0, 8.5), 5.0, 40.0, 330.0, false);
                let end = pts[0];
                self.fill(&[(end.0 + 3.5, end.1 - 2.0), (end.0 - 0.5, end.1 - 3.5), (end.0 + 0.5, end.1 + 2.5)], true);
                self.text((8.0, 8.5), "=", 8.0, true);
            }
            Import => {
                self.rect(2.0, 8.0, 14.0, 14.0, false);
                self.line(&[(2.0, 11.0), (14.0, 11.0)], false);
                self.line(&[(8.0, 8.0), (8.0, 14.0)], false);
                self.arrow((8.0, 0.5), (8.0, 7.0), true);
            }
            ImportMany => {
                self.rect(1.5, 9.0, 8.5, 14.5, false);
                self.rect(7.5, 9.0, 14.5, 14.5, false);
                self.arrow((4.0, 1.0), (4.0, 7.5), true);
                self.arrow((11.0, 1.0), (11.0, 7.5), true);
            }
            Print => {
                self.rect(3.5, 1.5, 12.5, 5.5, false);
                self.rect(1.5, 5.5, 14.5, 11.5, false);
                self.fill_rect(4.0, 9.0, 12.0, 14.5, true);
            }
            Duplicate => {
                self.rect(2.0, 2.0, 9.5, 10.5, false);
                self.rect(6.0, 5.5, 13.5, 14.0, true);
            }
            PanelLeft => {
                self.rect(2.0, 2.5, 14.0, 13.5, false);
                self.fill_rect(2.0, 2.5, 6.0, 13.5, true);
            }
            PanelRight => {
                self.rect(2.0, 2.5, 14.0, 13.5, false);
                self.fill_rect(10.0, 2.5, 14.0, 13.5, true);
            }
            LogLines => {
                self.rect(2.0, 2.5, 14.0, 13.5, false);
                for y in [5.5, 8.0, 10.5] {
                    self.line(&[(4.5, y), (11.5, y)], true);
                }
            }
            AddColumns => {
                self.rect(1.5, 3.0, 11.0, 13.0, false);
                self.fill_rect(1.5, 3.0, 11.0, 6.0, true);
                self.line(&[(5.0, 3.0), (5.0, 13.0)], false);
                self.line(&[(8.0, 3.0), (8.0, 13.0)], false);
                self.line(&[(12.5, 8.0), (15.5, 8.0)], true);
                self.line(&[(14.0, 6.5), (14.0, 9.5)], true);
            }
            Sigma => {
                self.line(&[(12.5, 3.0), (4.0, 3.0), (8.5, 8.0), (4.0, 13.0), (12.5, 13.0)], false);
            }
            SortUp | SortDown => {
                let up = icon == SortUp;
                let (a, b) = if up { (3.0, 12.0) } else { (12.0, 3.0) };
                self.arrow((4.0, a), (4.0, b), true);
                for (i, len) in [3.0, 6.0, 9.0].iter().enumerate() {
                    let y = if up { 12.0 - 4.0 * i as f32 } else { 4.0 + 4.0 * i as f32 };
                    self.line(&[(7.0, y), (7.0 + len, y)], false);
                }
            }
            Fx => {
                self.text((8.0, 8.0), "f(x)", 9.5, false);
                self.line(&[(2.0, 14.0), (14.0, 14.0)], true);
            }
            Filter => {
                self.fill(&[(2.0, 2.5), (14.0, 2.5), (9.0, 8.5), (9.0, 14.0), (7.0, 12.5), (7.0, 8.5)], true);
            }
            Letter(ch) => {
                self.text((8.0, 8.0), &ch.to_string(), 12.5, false);
                self.line(&[(3.0, 14.5), (13.0, 14.5)], true);
            }
            ErrorBar => {
                self.line(&[(8.0, 2.0), (8.0, 14.0)], false);
                self.line(&[(5.0, 2.0), (11.0, 2.0)], false);
                self.line(&[(5.0, 14.0), (11.0, 14.0)], false);
                self.dot((8.0, 8.0), 2.2, true);
            }
            Tag => {
                self.closed(&[(2.0, 8.0), (8.0, 2.0), (14.0, 2.0), (14.0, 8.0), (8.0, 14.0)], false);
                self.dot((11.0, 5.0), 1.4, true);
            }
            Disregard => {
                self.circle((8.0, 8.0), 5.5, false);
                self.line(&[(4.0, 12.0), (12.0, 4.0)], true);
            }
            Cut => {
                self.line(&[(4.0, 12.0), (11.0, 1.5)], false);
                self.line(&[(12.0, 12.0), (5.0, 1.5)], false);
                self.circle((4.0, 12.5), 2.0, true);
                self.circle((12.0, 12.5), 2.0, true);
            }
            Copy => {
                self.rect(2.0, 1.5, 9.5, 10.5, false);
                self.rect(6.0, 5.0, 13.5, 14.5, true);
            }
            Paste => {
                self.rect(2.5, 3.0, 13.5, 14.5, false);
                self.fill_rect(5.5, 1.0, 10.5, 4.5, true);
                self.line(&[(5.0, 8.0), (11.0, 8.0)], false);
                self.line(&[(5.0, 11.0), (11.0, 11.0)], false);
            }
            Undo | Redo => {
                let pen = Pen { mirror: icon == Redo, ..*self };
                let pts = pen.arc((8.0, 9.5), 4.5, -170.0, 20.0, false);
                let s = pts[0];
                pen.fill(&[(s.0 - 3.5, s.1 + 1.0), (s.0 + 3.0, s.1 + 1.0), (s.0 - 0.5, s.1 - 3.5)], true);
            }
            Cascade => {
                self.rect(1.5, 1.5, 10.0, 9.0, false);
                self.fill_rect(1.5, 1.5, 10.0, 3.5, false);
                self.rect(5.5, 6.0, 14.5, 14.0, true);
                self.fill_rect(5.5, 6.0, 14.5, 8.0, true);
            }
            TileH => {
                self.rect(2.0, 2.0, 14.0, 7.0, false);
                self.rect(2.0, 9.0, 14.0, 14.0, true);
            }
            TileV => {
                self.rect(2.0, 2.0, 7.0, 14.0, false);
                self.rect(9.0, 2.0, 14.0, 14.0, true);
            }
            Bold => {
                // egui has no bold face: the letter is drawn twice, a little apart.
                self.text((7.7, 8.0), "B", 13.0, false);
                self.text((8.4, 8.0), "B", 13.0, false);
            }
            Italic => {
                self.line(&[(10.0, 2.5), (6.0, 13.5)], false);
                self.line(&[(8.0, 2.5), (12.0, 2.5)], true);
                self.line(&[(4.0, 13.5), (8.0, 13.5)], true);
            }
            Underline => {
                self.line(&[(4.5, 2.5), (4.5, 8.0), (8.0, 11.0), (11.5, 8.0), (11.5, 2.5)], false);
                self.line(&[(3.0, 14.0), (13.0, 14.0)], true);
            }
            Superscript => {
                self.text((6.0, 10.0), "x", 11.0, false);
                self.text((12.5, 4.0), "2", 8.0, true);
            }
            Subscript => {
                self.text((6.0, 6.5), "x", 11.0, false);
                self.text((12.5, 12.5), "2", 8.0, true);
            }
            Greek => self.text((8.0, 8.0), "\u{3b1}\u{3b2}", 11.0, false),
            Bigger => {
                self.text((6.5, 9.0), "A", 12.0, false);
                self.arrow((13.0, 7.0), (13.0, 2.0), true);
            }
            Smaller => {
                self.text((6.5, 9.0), "A", 9.5, false);
                self.arrow((13.0, 9.0), (13.0, 14.0), true);
            }
            FontColor => {
                self.text((8.0, 7.5), "A", 12.0, false);
                self.fill_rect(3.0, 12.5, 13.0, 15.0, true);
            }
            FillColor => {
                self.closed(&[(3.0, 8.0), (8.0, 2.5), (12.5, 7.0), (7.5, 12.5)], false);
                self.fill_rect(2.5, 13.0, 13.5, 15.5, true);
            }
            LineColor => {
                self.line(&[(3.0, 11.5), (11.0, 2.5)], false);
                self.fill_rect(2.5, 13.0, 13.5, 15.5, true);
            }
            Pointer => {
                self.fill(
                    &[(4.0, 1.5), (4.0, 12.5), (7.0, 9.5), (9.5, 14.5), (11.0, 13.5), (8.5, 8.5), (12.0, 8.5)],
                    false,
                );
            }
            ZoomIn | ZoomOut => {
                self.circle((6.5, 6.5), 4.5, false);
                self.line(&[(10.0, 10.0), (14.5, 14.5)], false);
                self.line(&[(4.0, 6.5), (9.0, 6.5)], true);
                if icon == ZoomIn {
                    self.line(&[(6.5, 4.0), (6.5, 9.0)], true);
                }
            }
            WholePage => {
                self.rect(3.0, 1.5, 13.0, 14.5, false);
                self.rect(5.0, 4.0, 11.0, 12.0, true);
            }
            Rescale => {
                self.rect(5.0, 5.0, 11.0, 11.0, false);
                self.arrow((4.5, 4.5), (1.5, 1.5), true);
                self.arrow((11.5, 4.5), (14.5, 1.5), true);
                self.arrow((4.5, 11.5), (1.5, 14.5), true);
                self.arrow((11.5, 11.5), (14.5, 14.5), true);
            }
            Exchange => {
                self.arrow((2.0, 5.0), (13.0, 5.0), false);
                self.arrow((14.0, 11.0), (3.0, 11.0), true);
            }
            Layer | LayerTop | LayerRight | LayerBoth => {
                let top = matches!(icon, LayerTop | LayerBoth);
                let right = matches!(icon, LayerRight | LayerBoth);
                self.line(&[(2.0, 2.0), (2.0, 14.0), (14.0, 14.0)], false);
                if top {
                    self.line(&[(2.0, 2.0), (14.0, 2.0)], true);
                }
                if right {
                    self.line(&[(14.0, 2.0), (14.0, 14.0)], true);
                }
                if !top && !right {
                    self.line(&[(5.0, 11.0), (8.0, 7.0), (12.0, 9.0)], true);
                }
            }
            Inset => {
                self.rect(1.5, 1.5, 14.5, 14.5, false);
                self.rect(7.5, 2.5, 13.5, 8.5, true);
            }
            DataReader => {
                self.circle((8.0, 8.0), 4.0, false);
                self.line(&[(8.0, 1.0), (8.0, 15.0)], true);
                self.line(&[(1.0, 8.0), (15.0, 8.0)], true);
            }
            Text => {
                self.line(&[(3.0, 3.5), (13.0, 3.5)], false);
                self.line(&[(8.0, 3.5), (8.0, 13.5)], false);
                self.line(&[(6.0, 13.5), (10.0, 13.5)], true);
            }
            Arrow => self.arrow((3.0, 13.0), (13.0, 3.0), false),
            Line => {
                self.line(&[(2.5, 13.5), (13.5, 2.5)], false);
                self.dot((2.5, 13.5), 1.5, true);
                self.dot((13.5, 2.5), 1.5, true);
            }
            Rectangle => {
                self.rect(3.0, 4.0, 13.0, 12.0, false);
                for p in [(3.0, 4.0), (13.0, 4.0), (3.0, 12.0), (13.0, 12.0)] {
                    self.dot(p, 1.3, true);
                }
            }
            Hand => {
                self.closed(
                    &[
                        (4.0, 8.0),
                        (4.0, 4.0),
                        (6.0, 4.0),
                        (6.0, 2.5),
                        (8.0, 2.5),
                        (8.0, 4.0),
                        (10.0, 4.0),
                        (12.0, 7.0),
                        (12.0, 11.0),
                        (9.0, 14.5),
                        (6.0, 14.5),
                    ],
                    false,
                );
                self.line(&[(6.0, 8.0), (6.0, 11.0)], true);
            }
            PlotLine => {
                self.axes();
                self.line(&[(4.0, 11.0), (7.0, 6.0), (10.0, 9.0), (14.0, 3.0)], true);
            }
            PlotScatter => {
                self.axes();
                for p in [(5.0, 10.5), (7.0, 7.0), (9.0, 9.5), (11.5, 5.0), (13.0, 7.0)] {
                    self.dot(p, 1.5, true);
                }
            }
            PlotLineSymbol => {
                self.axes();
                let pts = [(5.0, 10.5), (8.0, 6.0), (10.5, 9.0), (13.5, 3.5)];
                self.line(&pts, true);
                for p in pts {
                    self.dot(p, 1.6, false);
                }
            }
            PlotColumn => {
                self.axes();
                self.fill_rect(4.0, 8.0, 6.5, 14.0, true);
                self.fill_rect(7.5, 4.0, 10.0, 14.0, true);
                self.fill_rect(11.0, 6.5, 13.5, 14.0, true);
            }
            PlotBar => {
                self.axes();
                self.fill_rect(2.0, 3.0, 9.0, 5.5, true);
                self.fill_rect(2.0, 6.5, 13.5, 9.0, true);
                self.fill_rect(2.0, 10.0, 6.5, 12.5, true);
            }
            PlotPie => {
                self.circle((8.0, 8.0), 6.0, false);
                self.fill(&[(8.0, 8.0), (8.0, 2.0), (12.2, 3.8), (14.0, 8.0)], true);
            }
            PlotArea => {
                self.axes();
                let top = [(3.0, 11.0), (6.5, 6.0), (10.0, 9.0), (14.0, 4.0)];
                for w in top.windows(2) {
                    self.fill(&[w[0], w[1], (w[1].0, 14.0), (w[0].0, 14.0)], true);
                }
            }
            PlotStackedArea => {
                self.axes();
                let top = [(3.0, 8.0), (6.5, 5.0), (10.0, 7.0), (14.0, 3.0)];
                let mid = [(3.0, 11.0), (6.5, 9.0), (10.0, 10.5), (14.0, 8.0)];
                for (w, v) in top.windows(2).zip(mid.windows(2)) {
                    self.fill(&[w[0], w[1], v[1], v[0]], false);
                    self.fill(&[v[0], v[1], (v[1].0, 14.0), (v[0].0, 14.0)], true);
                }
            }
            PlotHlc => {
                for (x, hi, lo, cl) in [(4.0, 3.0, 9.0, 7.0), (8.0, 5.0, 12.0, 8.0), (12.0, 2.0, 8.0, 4.0)] {
                    self.line(&[(x, hi), (x, lo)], false);
                    self.line(&[(x, cl), (x + 2.5, cl)], true);
                }
            }
            PlotBox => {
                self.rect(4.0, 5.0, 12.0, 11.0, false);
                self.line(&[(4.0, 8.0), (12.0, 8.0)], true);
                self.line(&[(8.0, 1.5), (8.0, 5.0)], false);
                self.line(&[(8.0, 11.0), (8.0, 14.5)], false);
            }
            PlotHistogram => {
                self.axes();
                self.fill_rect(3.0, 10.0, 6.0, 14.0, true);
                self.fill_rect(6.0, 4.0, 9.0, 14.0, true);
                self.fill_rect(9.0, 7.0, 12.0, 14.0, true);
                self.fill_rect(12.0, 11.0, 14.5, 14.0, true);
            }
            PlotBubble => {
                self.axes();
                self.circle((6.0, 10.0), 2.5, true);
                self.circle((11.0, 6.0), 3.8, true);
                self.dot((12.5, 12.0), 1.3, false);
            }
            PlotPolar => {
                self.circle((8.0, 8.0), 6.5, false);
                self.circle((8.0, 8.0), 3.2, false);
                self.line(&[(1.5, 8.0), (14.5, 8.0)], false);
                self.line(&[(8.0, 1.5), (8.0, 14.5)], false);
                self.line(&[(8.0, 8.0), (10.0, 6.0), (12.0, 7.0), (13.0, 3.5)], true);
            }
            PlotTernary => {
                self.closed(&[(8.0, 1.5), (14.5, 13.5), (1.5, 13.5)], false);
                self.dot((8.0, 9.0), 1.8, true);
            }
            PlotRadar => {
                self.closed(&[(8.0, 1.5), (14.0, 6.0), (11.5, 13.5), (4.5, 13.5), (2.0, 6.0)], false);
                self.closed(&[(8.0, 5.0), (11.0, 7.0), (10.0, 11.0), (6.0, 10.0), (5.0, 7.0)], true);
            }
            AlignLeft => {
                self.line(&[(2.5, 1.5), (2.5, 14.5)], false);
                self.fill_rect(4.0, 3.5, 14.0, 6.5, true);
                self.fill_rect(4.0, 9.5, 10.0, 12.5, true);
            }
            AlignRight => {
                self.line(&[(13.5, 1.5), (13.5, 14.5)], false);
                self.fill_rect(2.0, 3.5, 12.0, 6.5, true);
                self.fill_rect(6.0, 9.5, 12.0, 12.5, true);
            }
            AlignTop => {
                self.line(&[(1.5, 2.5), (14.5, 2.5)], false);
                self.fill_rect(3.5, 4.0, 6.5, 14.0, true);
                self.fill_rect(9.5, 4.0, 12.5, 10.0, true);
            }
            AlignBottom => {
                self.line(&[(1.5, 13.5), (14.5, 13.5)], false);
                self.fill_rect(3.5, 2.0, 6.5, 12.0, true);
                self.fill_rect(9.5, 6.0, 12.5, 12.0, true);
            }
            AlignCenterH => {
                self.line(&[(8.0, 1.5), (8.0, 14.5)], false);
                self.fill_rect(3.0, 3.5, 13.0, 6.5, true);
                self.fill_rect(5.0, 9.5, 11.0, 12.5, true);
            }
            AlignCenterV => {
                self.line(&[(1.5, 8.0), (14.5, 8.0)], false);
                self.fill_rect(3.5, 3.0, 6.5, 13.0, true);
                self.fill_rect(9.5, 5.0, 12.5, 11.0, true);
            }
            SameWidth => {
                self.rect(2.0, 3.0, 14.0, 6.0, true);
                self.rect(2.0, 10.0, 14.0, 13.0, true);
                self.arrow((8.0, 8.0), (14.0, 8.0), false);
                self.arrow((8.0, 8.0), (2.0, 8.0), false);
            }
            SameHeight => {
                self.rect(3.0, 2.0, 6.0, 14.0, true);
                self.rect(10.0, 2.0, 13.0, 14.0, true);
                self.arrow((8.0, 8.0), (8.0, 2.0), false);
                self.arrow((8.0, 8.0), (8.0, 14.0), false);
            }
            Front => {
                self.rect(2.0, 2.0, 10.0, 10.0, false);
                self.fill_rect(6.0, 6.0, 14.0, 14.0, true);
            }
            Back => {
                self.fill_rect(2.0, 2.0, 10.0, 10.0, true);
                self.rect(6.0, 6.0, 14.0, 14.0, false);
            }
            Group => {
                self.rect(1.5, 1.5, 14.5, 14.5, true);
                self.fill_rect(4.0, 4.0, 8.0, 8.0, false);
                self.fill_rect(8.0, 8.0, 12.0, 12.0, false);
            }
            Ungroup => {
                self.rect(2.0, 2.0, 7.0, 7.0, false);
                self.rect(9.0, 9.0, 14.0, 14.0, false);
                self.line(&[(9.0, 2.0), (14.0, 2.0), (14.0, 7.0)], true);
                self.line(&[(7.0, 14.0), (2.0, 14.0), (2.0, 9.0)], true);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::menus::{Entry, WindowKind};

    fn buttons() -> Vec<(Bar, &'static Button)> {
        Bar::ALL
            .iter()
            .flat_map(|bar| {
                items(*bar).iter().filter_map(move |i| match i {
                    Item::Button(b) => Some((*bar, b)),
                    _ => None,
                })
            })
            .collect()
    }

    fn walk(entries: &[Entry], f: &mut dyn FnMut(&menus::Item)) {
        for e in entries {
            match e {
                Entry::Item(i) => f(i),
                Entry::Submenu(s) => walk(&s.entries, f),
                _ => {}
            }
        }
    }

    #[test]
    fn every_button_with_a_command_sends_the_command_of_the_menu_entry_it_is_named_after() {
        for (bar, button) in buttons() {
            let Some(cmd) = &button.cmd else { continue };
            if is_tool(cmd) {
                assert!(button.menu.is_empty(), "{}: tools have no menu entry", button.tip);
                continue;
            }
            let mut found = false;
            for kind in [WindowKind::None, WindowKind::Workbook, WindowKind::Graph] {
                for m in menus::bar(kind) {
                    walk(&m.entries, &mut |item| {
                        let name = menus::plain(&item.label);
                        let name = name.trim_end_matches("...");
                        if name == button.menu && item.cmd.as_ref() == Some(cmd) {
                            found = true;
                        }
                    });
                }
            }
            assert!(found, "{} > {}: no menu entry {:?} with the command {cmd:?}", bar.name(), button.tip, button.menu);
        }
    }

    #[test]
    fn buttons_without_a_command_are_greyed_and_tools_are_only_on_the_tools_bar() {
        for (bar, button) in buttons() {
            assert!(!button.on, "{}: {}", bar.name(), button.tip);
            match &button.cmd {
                Some(cmd) if is_tool(cmd) => assert_eq!(bar, Bar::Tools, "{}", button.tip),
                Some(_) => assert!(!button.menu.is_empty(), "{}: {} has no menu name", bar.name(), button.tip),
                None => {}
            }
        }
    }

    #[test]
    fn the_bars_hold_origins_buttons_in_origins_order() {
        let tips = |bar| {
            items(bar)
                .iter()
                .filter_map(|i| match i {
                    Item::Button(b) => Some(b.tip),
                    _ => None,
                })
                .collect::<Vec<_>>()
        };
        assert_eq!(&tips(Bar::Graphs2D)[..5], ["Line", "Scatter", "Line + Symbol", "Column", "Bar"]);
        assert_eq!(&tips(Bar::Tools)[..4], ["Pointer", "Zoom In", "Zoom Out", "Rescale"]);
        assert_eq!(&tips(Bar::Tools)[4..], ["Data Reader", "Text", "Arrow", "Line", "Rectangle", "Pan"]);
        assert_eq!(&tips(Bar::Edit)[..3], ["Cut", "Copy", "Paste"]);
        assert_eq!(&tips(Bar::Standard)[..2], ["New Project", "New Workbook"]);
    }

    #[test]
    fn tooltips_are_distinct_within_a_bar() {
        for bar in Bar::ALL {
            let mut seen = HashSet::new();
            for (b, button) in buttons() {
                if b == bar {
                    assert!(seen.insert(button.tip), "{}: {} twice", bar.name(), button.tip);
                }
            }
        }
    }

    #[test]
    fn hiding_and_showing_a_bar_is_remembered() {
        let mut t = Toolbars::default();
        assert!(Bar::ALL.iter().all(|b| t.shown(*b)));
        t.set(Bar::Tools, false);
        assert!(!t.shown(Bar::Tools) && t.shown(Bar::Standard));
        assert!(t.any(&[Bar::Tools, Bar::Standard]) && !t.any(&[Bar::Tools]));
        t.set(Bar::Tools, true);
        assert!(t.shown(Bar::Tools));
    }
}
