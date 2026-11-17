//! Worksheet > Conditional Formatting > Highlight Min Max, Highlight Top Bottom, Conditional
//! Heatmap, Conditional Pareto and Conditional Outliers: the windows that add a rule to the active sheet for the selected cells (Origin's
//! `wcolorbycr` and `wcolortb`, and its Heatmap dialog). The rule is evaluated live by the grid
//! (see `sheaf_core::condformat`).

use super::frame::{DialogButton, button_row};
use super::graph_props::color_button;
use eframe::egui;
use sheaf_core::color::Color;
use sheaf_core::condformat::heatmap::{self, Heatmap, LevelScale, LevelStep, MiddlePosition, Mixing, Target};
use sheaf_core::condformat::{self, Colours, CondFormat, CondRange, Direction, OutlierMethod, Rule};
use sheaf_core::{Document, WindowId};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CondKind {
    MinMax,
    TopBottom,
    Heatmap,
    Pareto,
    Outliers,
}

/// The four modes of Highlight Top Bottom.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    TopItems,
    TopPercent,
    BottomItems,
    BottomPercent,
}

impl Mode {
    const ALL: [Mode; 4] = [Mode::TopItems, Mode::TopPercent, Mode::BottomItems, Mode::BottomPercent];

    fn from_parts(top: bool, percent: bool) -> Mode {
        match (top, percent) {
            (true, false) => Mode::TopItems,
            (true, true) => Mode::TopPercent,
            (false, false) => Mode::BottomItems,
            (false, true) => Mode::BottomPercent,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Mode::TopItems => "Top N Items",
            Mode::TopPercent => "Top N%",
            Mode::BottomItems => "Bottom N Items",
            Mode::BottomPercent => "Bottom N%",
        }
    }

    /// The same four modes as Conditional Pareto names them.
    fn pareto_label(self) -> &'static str {
        match self {
            Mode::TopItems => "Most Frequent N Items",
            Mode::TopPercent => "Most Frequent N%",
            Mode::BottomItems => "Least Frequent N Items",
            Mode::BottomPercent => "Least Frequent N%",
        }
    }

    fn parts(self) -> (bool, bool) {
        (matches!(self, Mode::TopItems | Mode::TopPercent), matches!(self, Mode::TopPercent | Mode::BottomPercent))
    }
}

/// One class of cells: whether its fill and its font colour are set, and the colours.
#[derive(Clone, Copy)]
struct ClassEdit {
    use_fill: bool,
    fill: Color,
    use_font: bool,
    font: Color,
}

impl ClassEdit {
    fn new(fill: Color, font: Color) -> Self {
        ClassEdit { use_fill: true, fill, use_font: true, font }
    }

    /// The edit of a rule's colours; a part that is off keeps `default` in its colour button.
    fn of(colours: Colours, default: (Color, Color)) -> Self {
        ClassEdit {
            use_fill: colours.fill.is_some(),
            fill: colours.fill.unwrap_or(default.0),
            use_font: colours.font.is_some(),
            font: colours.font.unwrap_or(default.1),
        }
    }

    fn colours(&self) -> Colours {
        Colours { fill: self.use_fill.then_some(self.fill), font: self.use_font.then_some(self.font) }
    }

    fn show(&mut self, ui: &mut egui::Ui, title: &str) {
        ui.label(title);
        ui.horizontal(|ui| {
            ui.checkbox(&mut self.use_fill, "Fill");
            ui.add_enabled_ui(self.use_fill, |ui| color_button(ui, &mut self.fill));
            ui.add_space(12.0);
            ui.checkbox(&mut self.use_font, "Font");
            ui.add_enabled_ui(self.use_font, |ui| color_button(ui, &mut self.font));
        });
    }
}

/// Origin's Middle Position modes of 3-Color Limited Mixing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MiddleMode {
    Percentile,
    Percent,
    Value,
}

/// The Conditional Heatmap window's settings.
struct HeatEdit {
    auto_rescale: bool,
    from: f64,
    to: f64,
    scale: LevelScale,
    by_count: bool,
    count: u32,
    increment: f64,
    use_first: bool,
    first: f64,
    target: Target,
    contrast_text: bool,
    three: bool,
    from_c: Color,
    middle_c: Color,
    to_c: Color,
    middle_mode: MiddleMode,
    middle: f64,
    flip: bool,
    use_below: bool,
    below: Color,
    use_above: bool,
    above: Color,
}

impl HeatEdit {
    fn new(extent: Option<(f64, f64)>) -> Self {
        let (from, to) = extent.unwrap_or((0.0, 1.0));
        let increment = (to - from) / f64::from(heatmap::DEFAULT_LEVELS);
        HeatEdit {
            auto_rescale: true,
            from,
            to,
            scale: LevelScale::Linear,
            by_count: true,
            count: heatmap::DEFAULT_LEVELS,
            increment: if increment > 0.0 { increment } else { 1.0 },
            use_first: false,
            first: from,
            target: Target::Fill,
            contrast_text: true,
            three: false,
            from_c: heatmap::DEFAULT_FROM,
            middle_c: heatmap::DEFAULT_MIDDLE,
            to_c: heatmap::DEFAULT_TO,
            middle_mode: MiddleMode::Percent,
            middle: 50.0,
            flip: false,
            use_below: false,
            below: condformat::MIN_FILL,
            use_above: false,
            above: condformat::MAX_FILL,
        }
    }

    /// The settings of an existing heatmap rule.
    fn of(heat: &Heatmap, extent: Option<(f64, f64)>) -> Self {
        let mut e = HeatEdit::new(extent);
        e.auto_rescale = heat.limits.is_none();
        if let Some((from, to)) = heat.limits {
            (e.from, e.to) = (from, to);
        }
        e.scale = heat.scale;
        match heat.step {
            LevelStep::Count(n) => (e.by_count, e.count) = (true, n),
            LevelStep::Increment(d) => (e.by_count, e.increment) = (false, d),
        }
        e.use_first = heat.first_level.is_some();
        e.first = heat.first_level.unwrap_or(e.from);
        e.target = heat.target;
        e.contrast_text = heat.contrast_text || heat.target == Target::Text;
        match heat.mixing {
            Mixing::Limited { from, to } => (e.three, e.from_c, e.to_c) = (false, from, to),
            Mixing::ThreeColor { from, middle, to, position } => {
                (e.three, e.from_c, e.middle_c, e.to_c) = (true, from, middle, to);
                (e.middle_mode, e.middle) = match position {
                    MiddlePosition::Percentile(v) => (MiddleMode::Percentile, v),
                    MiddlePosition::Percent(v) => (MiddleMode::Percent, v),
                    MiddlePosition::Value(v) => (MiddleMode::Value, v),
                };
            }
        }
        e.flip = heat.flip;
        e.use_below = heat.below.is_some();
        e.below = heat.below.unwrap_or(e.below);
        e.use_above = heat.above.is_some();
        e.above = heat.above.unwrap_or(e.above);
        e
    }

    fn rule(&self) -> Heatmap {
        let position = match self.middle_mode {
            MiddleMode::Percentile => MiddlePosition::Percentile(self.middle),
            MiddleMode::Percent => MiddlePosition::Percent(self.middle),
            MiddleMode::Value => MiddlePosition::Value(self.middle),
        };
        Heatmap {
            limits: (!self.auto_rescale).then_some((self.from, self.to)),
            scale: self.scale,
            step: if self.by_count { LevelStep::Count(self.count) } else { LevelStep::Increment(self.increment) },
            first_level: self.use_first.then_some(self.first),
            target: self.target,
            contrast_text: self.contrast_text && self.target == Target::Fill,
            mixing: if self.three {
                Mixing::ThreeColor { from: self.from_c, middle: self.middle_c, to: self.to_c, position }
            } else {
                Mixing::Limited { from: self.from_c, to: self.to_c }
            },
            flip: self.flip,
            below: (!self.auto_rescale && self.use_below).then_some(self.below),
            above: (!self.auto_rescale && self.use_above).then_some(self.above),
        }
    }

    fn show(&mut self, ui: &mut egui::Ui, extent: Option<(f64, f64)>) {
        ui.strong("Level");
        ui.checkbox(&mut self.auto_rescale, "Auto Rescale");
        ui.add_enabled_ui(!self.auto_rescale, |ui| {
            ui.horizontal(|ui| {
                ui.label("From");
                ui.add(egui::DragValue::new(&mut self.from).speed(0.1));
                ui.label("To");
                ui.add(egui::DragValue::new(&mut self.to).speed(0.1));
                if ui.add_enabled(extent.is_some(), egui::Button::new("Find Min/Max")).clicked()
                    && let Some((lo, hi)) = extent
                {
                    (self.from, self.to) = (lo, hi);
                }
            });
        });
        ui.horizontal(|ui| {
            ui.label("Type");
            let name = |s: LevelScale| match s {
                LevelScale::Linear => "Linear",
                LevelScale::Log10 => "Log10",
            };
            egui::ComboBox::from_id_salt("heat_scale").selected_text(name(self.scale)).show_ui(ui, |ui| {
                for s in [LevelScale::Linear, LevelScale::Log10] {
                    ui.selectable_value(&mut self.scale, s, name(s));
                }
            });
        });
        ui.horizontal(|ui| {
            ui.radio_value(&mut self.by_count, false, "By Increment");
            ui.radio_value(&mut self.by_count, true, "By Count");
            if self.by_count {
                ui.add(egui::DragValue::new(&mut self.count).range(1..=heatmap::MAX_LEVELS as u32));
            } else {
                ui.add(egui::DragValue::new(&mut self.increment).range(f64::MIN_POSITIVE..=f64::MAX).speed(0.01));
            }
        });
        ui.horizontal(|ui| {
            ui.checkbox(&mut self.use_first, "First Level");
            ui.add_enabled(self.use_first, egui::DragValue::new(&mut self.first).speed(0.1));
        });
        ui.add_space(6.0);
        ui.strong("Colors");
        ui.horizontal(|ui| {
            ui.label("Format Type");
            ui.radio_value(&mut self.target, Target::Fill, "Fill Background");
            ui.radio_value(&mut self.target, Target::Text, "Color Text");
        });
        ui.add_enabled(
            self.target == Target::Fill,
            egui::Checkbox::new(&mut self.contrast_text, "Contrast Color for Text"),
        );
        ui.label("Color Generation");
        ui.radio_value(&mut self.three, false, "Limited Mixing");
        ui.radio_value(&mut self.three, true, "3-Color Limited Mixing");
        // Not built yet: Origin's complementary-colour mixing and palettes.
        ui.add_enabled(false, egui::RadioButton::new(false, "Introducing Other Colors in Mixing"));
        ui.add_enabled(false, egui::RadioButton::new(false, "Load Palette"));
        ui.horizontal(|ui| {
            ui.label("From");
            color_button(ui, &mut self.from_c);
            ui.add_enabled_ui(self.three, |ui| {
                ui.label("Middle");
                color_button(ui, &mut self.middle_c);
            });
            ui.label("To");
            color_button(ui, &mut self.to_c);
        });
        ui.add_enabled_ui(self.three, |ui| {
            ui.horizontal(|ui| {
                ui.label("Middle Position");
                let name = |m: MiddleMode| match m {
                    MiddleMode::Percentile => "By Percentile",
                    MiddleMode::Percent => "By Percent",
                    MiddleMode::Value => "By Value",
                };
                egui::ComboBox::from_id_salt("heat_middle").selected_text(name(self.middle_mode)).show_ui(ui, |ui| {
                    for m in [MiddleMode::Percentile, MiddleMode::Percent, MiddleMode::Value] {
                        ui.selectable_value(&mut self.middle_mode, m, name(m));
                    }
                });
                ui.add(egui::DragValue::new(&mut self.middle).speed(0.5));
            });
        });
        ui.checkbox(&mut self.flip, "Flip");
        ui.add_enabled_ui(!self.auto_rescale, |ui| {
            ui.horizontal(|ui| {
                ui.checkbox(&mut self.use_below, "Below");
                ui.add_enabled_ui(self.use_below, |ui| color_button(ui, &mut self.below));
                ui.add_space(12.0);
                ui.checkbox(&mut self.use_above, "Above");
                ui.add_enabled_ui(self.use_above, |ui| color_button(ui, &mut self.above));
            });
        });
    }
}

/// A number as the windows write it (`10`, `1.5`).
fn num(v: f64) -> String {
    format!("{v}")
}

/// What a rule does, in a few words (the Rule column of the Conditional Format Manager).
pub fn rule_text(rule: &Rule) -> String {
    match rule {
        Rule::MinMax { dir: Direction::RowWise, .. } => "Highlight Min Max, row-wise".into(),
        Rule::MinMax { dir: Direction::ColumnWise, .. } => "Highlight Min Max, column-wise".into(),
        Rule::TopBottom { top, n, percent, .. } => Mode::from_parts(*top, *percent).label().replacen('N', &num(*n), 1),
        Rule::Pareto { most, n, percent, .. } => {
            Mode::from_parts(*most, *percent).pareto_label().replacen(" N", &format!(" {}", num(*n)), 1)
        }
        Rule::Heatmap(heat) => match heat.step {
            LevelStep::Count(n) => format!("Heatmap, {n} levels"),
            LevelStep::Increment(d) => format!("Heatmap, increment {}", num(d)),
        },
        Rule::Outliers { method: OutlierMethod::Iqr, coef, .. } => format!("Outliers, IQR x {}", num(*coef)),
        Rule::Outliers { method: OutlierMethod::Sd, coef, .. } => format!("Outliers, SD x {}", num(*coef)),
    }
}

/// A rule's cells in words: the columns' short names in sheet order and the rows (1-based), for
/// example `B, C; rows 1 to end`.
pub fn range_text(sheet: &sheaf_core::Worksheet, range: &CondRange) -> String {
    let names: Vec<&str> =
        sheet.columns.iter().filter(|c| range.columns.contains(&c.id)).map(|c| c.short_name.as_str()).collect();
    let last = range.last_row.map_or_else(|| "end".to_string(), |l| (l + 1).to_string());
    format!("{}; rows {} to {}", names.join(", "), range.first_row + 1, last)
}

/// The smallest and largest number in the range's cells (masked and text cells left out).
fn range_extent(doc: &Document, book: WindowId, sheet: usize, range: &CondRange) -> Option<(f64, f64)> {
    let s = doc.project().workbook(book)?.sheets.get(sheet)?;
    let rows = s.row_count();
    let last = range.last_row.map_or(rows, |l| l.saturating_add(1)).min(rows);
    let (mut lo, mut hi) = (f64::INFINITY, f64::NEG_INFINITY);
    for c in s.columns.iter().filter(|c| range.columns.contains(&c.id)) {
        let Some(v) = c.numeric() else { continue };
        let mask = c.active_mask();
        for r in range.first_row.min(last)..last {
            if let Some(x) = v.get(r).copied().filter(|x| x.is_finite())
                && !mask.is_some_and(|m| m.contains(&r))
            {
                lo = lo.min(x);
                hi = hi.max(x);
            }
        }
    }
    (lo <= hi).then_some((lo, hi))
}

pub struct CondFormatDialog {
    pub book: WindowId,
    sheet: usize,
    kind: CondKind,
    range: CondRange,
    name: String,
    dir: Direction,
    max: ClassEdit,
    min: ClassEdit,
    mode: Mode,
    n: f64,
    hit: ClassEdit,
    heat: HeatEdit,
    extent: Option<(f64, f64)>,
    method: OutlierMethod,
    coef: f64,
    /// Conditional Format Manager > Edit Rule: the rule's place in the list and the rule as it was,
    /// so OK replaces it in place, and does nothing when the list has changed meanwhile.
    editing: Option<(usize, CondFormat)>,
}

impl CondFormatDialog {
    pub fn new(doc: &Document, book: WindowId, kind: CondKind, range: CondRange) -> Option<Self> {
        let b = doc.project().workbook(book)?;
        let sheet = b.active_sheet.min(b.sheets.len().checked_sub(1)?);
        let name = condformat::default_name(&b.sheets[sheet], &range);
        let extent = range_extent(doc, book, sheet, &range);
        Some(CondFormatDialog {
            book,
            sheet,
            kind,
            range,
            name,
            dir: Direction::RowWise,
            max: ClassEdit::new(condformat::MAX_FILL, condformat::MAX_FONT),
            min: ClassEdit::new(condformat::MIN_FILL, condformat::MIN_FONT),
            mode: Mode::TopItems,
            n: condformat::DEFAULT_N,
            hit: ClassEdit::new(condformat::TOP_FILL, condformat::TOP_FONT),
            heat: HeatEdit::new(extent),
            extent,
            method: OutlierMethod::Iqr,
            coef: condformat::rank::DEFAULT_COEF,
            editing: None,
        })
    }

    /// The window of an existing rule (sheet `sheet`, place `index`), filled with its settings.
    pub fn edit(doc: &Document, book: WindowId, sheet: usize, index: usize) -> Option<Self> {
        let original = doc.project().workbook(book)?.sheets.get(sheet)?.cond_formats.get(index)?.clone();
        let kind = match original.rule {
            Rule::MinMax { .. } => CondKind::MinMax,
            Rule::TopBottom { .. } => CondKind::TopBottom,
            Rule::Heatmap(_) => CondKind::Heatmap,
            Rule::Pareto { .. } => CondKind::Pareto,
            Rule::Outliers { .. } => CondKind::Outliers,
        };
        let mut d = CondFormatDialog::new(doc, book, kind, original.range.clone())?;
        d.sheet = sheet;
        d.extent = range_extent(doc, book, sheet, &original.range);
        d.name = original.name.clone();
        let top = (condformat::TOP_FILL, condformat::TOP_FONT);
        let bottom = (condformat::BOTTOM_FILL, condformat::BOTTOM_FONT);
        match &original.rule {
            Rule::MinMax { dir, max, min } => {
                d.dir = *dir;
                d.max = ClassEdit::of(*max, (condformat::MAX_FILL, condformat::MAX_FONT));
                d.min = ClassEdit::of(*min, (condformat::MIN_FILL, condformat::MIN_FONT));
            }
            Rule::TopBottom { top: t, n, percent, colours } | Rule::Pareto { most: t, n, percent, colours } => {
                d.mode = Mode::from_parts(*t, *percent);
                d.n = *n;
                d.hit = ClassEdit::of(*colours, if *t { top } else { bottom });
            }
            Rule::Heatmap(heat) => d.heat = HeatEdit::of(heat, d.extent),
            Rule::Outliers { method, coef, colours } => {
                d.method = *method;
                d.coef = *coef;
                d.hit = ClassEdit::of(*colours, top);
            }
        }
        d.editing = Some((index, original));
        Some(d)
    }

    fn rule(&self) -> Rule {
        match self.kind {
            CondKind::MinMax => Rule::MinMax { dir: self.dir, max: self.max.colours(), min: self.min.colours() },
            CondKind::TopBottom => {
                let (top, percent) = self.mode.parts();
                Rule::TopBottom { top, n: self.n, percent, colours: self.hit.colours() }
            }
            CondKind::Heatmap => Rule::Heatmap(self.heat.rule()),
            CondKind::Pareto => {
                let (most, percent) = self.mode.parts();
                Rule::Pareto { most, n: self.n, percent, colours: self.hit.colours() }
            }
            CondKind::Outliers => Rule::Outliers { method: self.method, coef: self.coef, colours: self.hit.colours() },
        }
    }

    /// Switches the default top colours to the bottom ones (and back) when the mode changes
    /// between top and bottom, unless the colours were changed.
    fn follow_default_colours(&mut self, before: Mode) {
        if self.mode == before || self.mode.parts().0 == before.parts().0 {
            return;
        }
        let pair = |top: bool| {
            if top {
                (condformat::TOP_FILL, condformat::TOP_FONT)
            } else {
                (condformat::BOTTOM_FILL, condformat::BOTTOM_FONT)
            }
        };
        let was = pair(before.parts().0);
        if self.hit.fill == was.0 && self.hit.font == was.1 {
            let now = pair(self.mode.parts().0);
            self.hit = ClassEdit::new(now.0, now.1);
        }
    }

    /// Draws the window; false when it closed.
    pub fn show(&mut self, ctx: &egui::Context, doc: &mut Document) -> bool {
        if doc.project().workbook(self.book).and_then(|b| b.sheets.get(self.sheet)).is_none() {
            return false;
        }
        let title = match self.kind {
            CondKind::MinMax => "Highlight Min Max",
            CondKind::TopBottom => "Highlight Top Bottom",
            CondKind::Heatmap => "Conditional Heatmap",
            CondKind::Pareto => "Conditional Pareto",
            CondKind::Outliers => "Conditional Outliers",
        };
        let mut open = true;
        let mut pressed = None;
        egui::Window::new(title)
            .id(egui::Id::new("cond_format"))
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Name");
                    ui.add(egui::TextEdit::singleline(&mut self.name).desired_width(160.0));
                });
                ui.add_space(4.0);
                match self.kind {
                    CondKind::MinMax => {
                        ui.horizontal(|ui| {
                            ui.label("Direction");
                            ui.radio_value(&mut self.dir, Direction::RowWise, "Row-wise");
                            ui.radio_value(&mut self.dir, Direction::ColumnWise, "Column-wise");
                        });
                        ui.add_space(4.0);
                        self.max.show(ui, "Maximum");
                        self.min.show(ui, "Minimum");
                    }
                    CondKind::TopBottom => {
                        ui.horizontal(|ui| {
                            ui.label("Mode");
                            let before = self.mode;
                            egui::ComboBox::from_id_salt("cond_mode").selected_text(self.mode.label()).show_ui(
                                ui,
                                |ui| {
                                    for m in Mode::ALL {
                                        ui.selectable_value(&mut self.mode, m, m.label());
                                    }
                                },
                            );
                            // The default colours follow top or bottom until they are changed.
                            self.follow_default_colours(before);
                        });
                        ui.horizontal(|ui| {
                            ui.label("N");
                            ui.add(egui::DragValue::new(&mut self.n).range(0.0..=1.0e9).speed(0.1));
                            ui.weak(if self.mode.parts().1 { "percent of the numbers" } else { "items" });
                        });
                        ui.add_space(4.0);
                        self.hit.show(ui, "Highlighted cells");
                        ui.weak("Each column of the selection is ranked on its own.");
                    }
                    CondKind::Heatmap => self.heat.show(ui, self.extent),
                    CondKind::Pareto => {
                        ui.horizontal(|ui| {
                            ui.label("Rule");
                            let before = self.mode;
                            egui::ComboBox::from_id_salt("cond_pareto")
                                .selected_text(self.mode.pareto_label())
                                .show_ui(ui, |ui| {
                                    for m in Mode::ALL {
                                        ui.selectable_value(&mut self.mode, m, m.pareto_label());
                                    }
                                });
                            self.follow_default_colours(before);
                        });
                        ui.horizontal(|ui| {
                            ui.label("N");
                            ui.add(egui::DragValue::new(&mut self.n).range(0.0..=1.0e9).speed(0.1));
                            ui.weak(if self.mode.parts().1 { "percent of the cells" } else { "items" });
                        });
                        ui.add_space(4.0);
                        self.hit.show(ui, "Highlighted cells");
                        ui.weak("Items are counted over the whole selection, text included.");
                    }
                    CondKind::Outliers => {
                        ui.horizontal(|ui| {
                            ui.radio_value(&mut self.method, OutlierMethod::Iqr, "IQR");
                            ui.radio_value(&mut self.method, OutlierMethod::Sd, "SD");
                        });
                        ui.horizontal(|ui| {
                            ui.label("Coef");
                            egui::ComboBox::from_id_salt("cond_coef").selected_text(format!("{}", self.coef)).show_ui(
                                ui,
                                |ui| {
                                    for c in [1.0, 1.5, 2.0, 3.0] {
                                        ui.selectable_value(&mut self.coef, c, format!("{c}"));
                                    }
                                },
                            );
                            ui.add(egui::DragValue::new(&mut self.coef).range(0.0..=1.0e6).speed(0.05));
                        });
                        ui.add_space(4.0);
                        self.hit.show(ui, "Outliers");
                        ui.weak("Each column of the selection is checked on its own, as one box of a box chart.");
                    }
                }
                ui.add_space(6.0);
                pressed = button_row(ui, false);
            });
        match pressed {
            Some(DialogButton::Ok) => {
                let (book, sheet) = (self.book, self.sheet);
                let range = self.range.clone();
                let rule = self.rule();
                let name = self.name.trim().to_string();
                let label = match self.kind {
                    CondKind::MinMax => "Highlight min max",
                    CondKind::TopBottom => "Highlight top bottom",
                    CondKind::Heatmap => "Conditional heatmap",
                    CondKind::Pareto => "Conditional pareto",
                    CondKind::Outliers => "Conditional outliers",
                };
                if let Some((index, original)) = self.editing.take() {
                    let current = doc.project().workbook(book).and_then(|b| b.sheets.get(sheet));
                    if current.and_then(|s| s.cond_formats.get(index)) != Some(&original) {
                        return false;
                    }
                    let mut edited = CondFormat { rule, ..original };
                    if !name.is_empty() {
                        edited.name = name;
                    }
                    if current.and_then(|s| s.cond_formats.get(index)) != Some(&edited) {
                        doc.apply("Edit conditional format", |p| {
                            if let Some(slot) = p
                                .workbook_mut(book)
                                .and_then(|b| b.sheets.get_mut(sheet))
                                .and_then(|s| s.cond_formats.get_mut(index))
                            {
                                *slot = edited;
                            }
                        });
                    }
                    return false;
                }
                doc.apply(label, |p| {
                    if let Some(s) = p.workbook_mut(book).and_then(|b| b.sheets.get_mut(sheet)) {
                        let mut rule = CondFormat::new(s, range, rule);
                        if !name.is_empty() {
                            rule.name = name;
                        }
                        // The newest rule goes on top, so it wins where rules overlap.
                        s.cond_formats.insert(0, rule);
                    }
                });
                return false;
            }
            Some(DialogButton::Cancel) => return false,
            _ => {}
        }
        open
    }
}
