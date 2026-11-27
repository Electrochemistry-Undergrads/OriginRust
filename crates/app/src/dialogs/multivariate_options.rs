//! The option controls of Statistics > Multivariate Analysis > Principal Component Analysis (and the other
//! multivariate and ROC tools as they arrive). The window frame is the shared one of `analysis.rs`.

use super::analysis::AnalysisKind;
use eframe::egui;
use sheaf_core::analysis::{
    AcfSpec, AdfTerms, ArimaSpec, BinRule, BinScatterSpec, CapModel, CapabilitySpec, ChangeKind, ChangePenalty,
    ChangePointSpec, CoxSpec, Damping, DecomposeModel, DecomposeSpec, ExpSmoothSpec, FactorialDesignSpec,
    FactorialEffectsSpec, ForestTask, GLimits, KaplanMeierSpec, KmeansSpec, KpssNull, LifeDist, LifeDistArbitrarySpec,
    LifeDistSpec, LifeRegressionSpec, ManhattanSpec, MixtureDesignSpec, MixtureKind, MonteCarloSpec, PcaSpec, PivotBy,
    PivotPeriod, PivotStat, QpcrCurveSpec, QpcrSpec, QuantRegSpec, RandomForestSpec, RareChartKind, RareChartSpec,
    RocSpec, SmoothModel, SplitPlotDesignSpec, SplitPlotEffectsSpec, StabilitySpec, StationaritySpec, TDistribution,
    TimePivotSpec, Tool, TurnbullSpec,
};

/// Comma-separated numbers typed in a box (T386); empty text is no number.
fn numbers(text: &str, what: &str) -> Result<Vec<f64>, String> {
    text.split([',', ';'])
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .map(|t| t.parse::<f64>().map_err(|_| format!("the {what} '{t}' is not a number")))
        .collect()
}

/// What the windows keep between frames.
#[derive(Default)]
pub struct MultiOptions {
    pca: PcaSpec,
    roc: RocSpec,
    quantreg: QuantRegSpec,
    changepoint: ChangePointSpec,
    binscatter: BinScatterSpec,
    timepivot: TimePivotSpec,
    manhattan: ManhattanSpec,
    acf: AcfSpec,
    decompose: DecomposeSpec,
    stationarity: StationaritySpec,
    expsmooth: ExpSmoothSpec,
    arima: ArimaSpec,
    forest: RandomForestSpec,
    design: FactorialDesignSpec,
    /// Mixture Design (T386), with its totals and lower bounds as typed (comma separated).
    mixture: MixtureDesignSpec,
    /// Monte Carlo Simulation (T393): the inputs as typed (one `name = dist(...)` per line) and the
    /// specification limits as typed (empty: none).
    monte_carlo: MonteCarloSpec,
    mc_inputs: Option<String>,
    mc_lsl: String,
    mc_usl: String,
    /// The optimisation (T490): the controls as typed (`name = low:high, ...`; empty: none), and whether the
    /// output aims at the target as typed instead of the highest Cpk.
    mc_controls: String,
    mc_aim: bool,
    mc_target: String,
    mixture_totals: Option<String>,
    mixture_lower: Option<String>,
    factorial: FactorialEffectsSpec,
    split_design: SplitPlotDesignSpec,
    split_effects: SplitPlotEffectsSpec,
    rare: RareChartSpec,
    capability: CapabilitySpec,
    stability: StabilitySpec,
    life: LifeDistSpec,
    life_any: LifeDistArbitrarySpec,
    turnbull: TurnbullSpec,
    life_reg: LifeRegressionSpec,
    /// The percent list and covariate values of the life-data regression's percentiles, as typed (T437).
    life_reg_percentiles: String,
    life_reg_at: String,
    cox: CoxSpec,
    /// The Cox window's covariate values for an extra survival curve, as typed (T457).
    cox_at: String,
    qpcr: QpcrSpec,
    qpcr_curve: QpcrCurveSpec,
    /// The value of the Manual penalty while another penalty is chosen.
    manual_penalty: Option<f64>,
    /// The quantiles as typed (comma separated), read when the tool runs.
    quantiles: Option<String>,
    km: KaplanMeierSpec,
    kmeans: KmeansSpec,
}

impl MultiOptions {
    /// Sets the Seasonal Decomposition period (for tests).
    #[cfg(test)]
    pub fn set_decompose_period(&mut self, period: usize) {
        self.decompose.period = period;
    }

    /// The quantiles as the window shows them.
    fn quantiles_text(&self) -> String {
        self.quantiles
            .clone()
            .unwrap_or_else(|| self.quantreg.quantiles.iter().map(|q| q.to_string()).collect::<Vec<_>>().join(", "))
    }

    /// The tool the window runs.
    pub fn tool(&self, kind: AnalysisKind) -> Result<Tool, String> {
        match kind {
            AnalysisKind::Pca => {
                if self.pca.components == 0 {
                    return Err("the number of components must be at least 1".to_string());
                }
                Ok(Tool::Pca(self.pca))
            }
            AnalysisKind::Roc => {
                if self.roc.positive.trim().is_empty() {
                    return Err("give the value of the state column that is the positive state".to_string());
                }
                Ok(Tool::Roc(self.roc.clone()))
            }
            AnalysisKind::QuantReg => {
                let text = self.quantiles_text();
                let quantiles: Vec<f64> = text
                    .split(',')
                    .map(str::trim)
                    .filter(|t| !t.is_empty())
                    .map(|t| t.parse::<f64>().map_err(|_| format!("\"{t}\" is not a number")))
                    .collect::<Result<_, _>>()?;
                if quantiles.is_empty() {
                    return Err("give at least one quantile".to_string());
                }
                if let Some(bad) = quantiles.iter().find(|t| !(**t > 0.0 && **t < 1.0)) {
                    return Err(format!("a quantile must lie between 0 and 1 (got {bad})"));
                }
                Ok(Tool::QuantReg(QuantRegSpec { quantiles, ..self.quantreg.clone() }))
            }
            AnalysisKind::TimePivot => Ok(Tool::TimePivot(self.timepivot)),
            AnalysisKind::LifeDistArbitrary => Ok(Tool::LifeDistArbitrary(self.life_any)),
            AnalysisKind::Turnbull => Ok(Tool::Turnbull(self.turnbull)),
            AnalysisKind::Qpcr => Ok(Tool::Qpcr(self.qpcr.clone())),
            AnalysisKind::QpcrCurve => Ok(Tool::QpcrCurve(self.qpcr_curve)),
            AnalysisKind::Cox => {
                if self.cox.censor.trim().is_empty() {
                    return Err("give the censoring value".to_string());
                }
                let mut spec = self.cox.clone();
                spec.at = if self.cox_at.trim().is_empty() {
                    None
                } else {
                    Some(
                        self.cox_at
                            .split([',', ' ', ';'])
                            .filter(|v| !v.trim().is_empty())
                            .map(|v| v.trim().parse::<f64>().map_err(|_| format!("'{}' is not a number", v.trim())))
                            .collect::<Result<Vec<f64>, String>>()?,
                    )
                };
                Ok(Tool::Cox(spec))
            }
            AnalysisKind::LifeRegression => {
                if self.life_reg.censor.trim().is_empty() {
                    return Err("give the censoring value".to_string());
                }
                let numbers = |text: &str, what: &str| -> Result<Vec<f64>, String> {
                    text.split([',', ' ', ';'])
                        .filter(|t| !t.trim().is_empty())
                        .map(|t| t.trim().parse::<f64>().map_err(|_| format!("{what}: '{}' is not a number", t.trim())))
                        .collect()
                };
                let mut spec = self.life_reg.clone();
                if !self.life_reg_percentiles.trim().is_empty() {
                    spec.percentiles =
                        numbers(&self.life_reg_percentiles, "percentiles")?.iter().map(|p| p / 100.0).collect();
                }
                spec.at = if self.life_reg_at.trim().is_empty() {
                    None
                } else {
                    Some(numbers(&self.life_reg_at, "covariate values")?)
                };
                Ok(Tool::LifeRegression(spec))
            }
            AnalysisKind::LifeDist => {
                if self.life.censor.trim().is_empty() {
                    return Err("give the censoring value".to_string());
                }
                Ok(Tool::LifeDist(self.life.clone()))
            }
            AnalysisKind::Stability => {
                let s = &self.stability;
                if s.lower.is_none() && s.upper.is_none() {
                    return Err("give a lower or an upper specification limit".to_string());
                }
                Ok(Tool::Stability(*s))
            }
            AnalysisKind::Capability => {
                let c = &self.capability;
                if c.lsl.is_none() && c.usl.is_none() {
                    return Err("give a lower or an upper specification limit".to_string());
                }
                Ok(Tool::Capability(*c))
            }
            AnalysisKind::RareChart => {
                if self.rare.sigma.is_nan() || self.rare.sigma <= 0.0 {
                    return Err("the number of sigmas must be above 0".to_string());
                }
                Ok(Tool::RareChart(self.rare))
            }
            AnalysisKind::Stationarity => Ok(Tool::Stationarity(self.stationarity)),
            AnalysisKind::FactorialDesign => Ok(Tool::FactorialDesign(self.design)),
            AnalysisKind::MixtureDesign => {
                let mut spec = self.mixture.clone();
                if let Some(t) = &self.mixture_totals {
                    spec.totals = numbers(t, "total")?;
                    if spec.totals.is_empty() {
                        spec.totals = vec![1.0];
                    }
                }
                if let Some(t) = &self.mixture_lower {
                    spec.lower = numbers(t, "lower bound")?;
                }
                Ok(Tool::MixtureDesign(spec))
            }
            AnalysisKind::MonteCarlo => {
                let mut spec = self.monte_carlo.clone();
                if let Some(text) = &self.mc_inputs {
                    spec.inputs = text
                        .lines()
                        .filter(|l| !l.split('#').next().unwrap_or("").trim().is_empty())
                        .map(sheaf_core::analysis::parse_monte_carlo_input)
                        .collect::<Result<_, _>>()?;
                }
                let limit = |t: &str, what: &str| -> Result<Option<f64>, String> {
                    let t = t.trim();
                    if t.is_empty() {
                        Ok(None)
                    } else {
                        t.parse::<f64>().map(Some).map_err(|_| format!("the {what} '{t}' is not a number"))
                    }
                };
                spec.lsl = limit(&self.mc_lsl, "lower specification limit")?;
                spec.usl = limit(&self.mc_usl, "upper specification limit")?;
                spec.optimize = sheaf_core::analysis::parse_monte_carlo_controls(&self.mc_controls)?;
                spec.goal = if self.mc_aim {
                    let t = limit(&self.mc_target, "target")?.ok_or("give the target the output aims at")?;
                    sheaf_core::analysis::McGoal::Target(t)
                } else {
                    sheaf_core::analysis::McGoal::Capability
                };
                Ok(Tool::MonteCarlo(spec))
            }
            AnalysisKind::FactorialEffects => Ok(Tool::FactorialEffects(self.factorial)),
            AnalysisKind::SplitPlotDesign => Ok(Tool::SplitPlotDesign(self.split_design)),
            AnalysisKind::SplitPlotEffects => Ok(Tool::SplitPlotEffects(self.split_effects)),
            AnalysisKind::RandomForest => {
                if self.forest.trees == 0 {
                    return Err("grow at least one tree".to_string());
                }
                Ok(Tool::RandomForest(self.forest))
            }
            AnalysisKind::Arima => {
                let s = &self.arima;
                if s.auto.is_none() && s.p + s.q + s.seasonal.p + s.seasonal.q == 0 && !s.constant {
                    return Err("the model has nothing to fit: give p, q or a constant".to_string());
                }
                Ok(Tool::Arima(*s))
            }
            AnalysisKind::ExpSmooth => {
                let s = &self.expsmooth;
                if s.horizon == 0 {
                    return Err("forecast at least one step".to_string());
                }
                Ok(Tool::ExpSmooth(*s))
            }
            AnalysisKind::Decompose => {
                if self.decompose.period < 2 {
                    return Err("the period must be at least 2".to_string());
                }
                Ok(Tool::Decompose(self.decompose))
            }
            AnalysisKind::Acf => {
                if !(self.acf.confidence > 0.0 && self.acf.confidence < 1.0) {
                    return Err("the confidence level must lie between 0 and 1".to_string());
                }
                Ok(Tool::Acf(self.acf))
            }
            AnalysisKind::Manhattan => {
                if !(self.manhattan.threshold > 0.0 && self.manhattan.threshold < 1.0) {
                    return Err("the significance level must lie between 0 and 1".to_string());
                }
                Ok(Tool::Manhattan(self.manhattan))
            }
            AnalysisKind::BinScatter => {
                let (BinRule::EqualWidth(n) | BinRule::EqualCount(n)) = self.binscatter.rule;
                if n == 0 {
                    return Err("the number of bins must be at least 1".to_string());
                }
                Ok(Tool::BinScatter(self.binscatter))
            }
            AnalysisKind::ChangePoint => {
                let need = if self.changepoint.kind == ChangeKind::Mean { 1 } else { 2 };
                if self.changepoint.min_segment < need {
                    return Err(format!("the minimum segment length must be at least {need}"));
                }
                Ok(Tool::ChangePoint(self.changepoint))
            }
            AnalysisKind::Kmeans => {
                if self.kmeans.clusters < 2 {
                    return Err("the number of clusters must be at least 2".to_string());
                }
                Ok(Tool::Kmeans(self.kmeans))
            }
            AnalysisKind::KaplanMeier => {
                if self.km.censor.trim().is_empty() {
                    return Err("give the censoring value".to_string());
                }
                Ok(Tool::KaplanMeier(self.km.clone()))
            }
            _ => unreachable!("only the multivariate tools come here"),
        }
    }

    pub fn options(&mut self, ui: &mut egui::Ui, kind: AnalysisKind) {
        match kind {
            AnalysisKind::Pca => {
                let s = &mut self.pca;
                ui.label("Every chosen column is a variable, every row a case (at least two columns).");
                ui.horizontal(|ui| {
                    ui.label("Analyze");
                    ui.radio_value(&mut s.correlation, true, "Correlation matrix");
                    ui.radio_value(&mut s.correlation, false, "Covariance matrix");
                });
                ui.horizontal(|ui| {
                    ui.label("Missing values");
                    ui.radio_value(&mut s.pairwise, false, "Exclude listwise");
                    ui.radio_value(&mut s.pairwise, true, "Exclude pairwise");
                });
                ui.horizontal(|ui| {
                    ui.label("Number of components");
                    ui.add(egui::DragValue::new(&mut s.components).range(1..=1000));
                });
                ui.checkbox(&mut s.standardize_scores, "Standardize the scores (unit variance)");
                ui.checkbox(&mut s.scores, "Write the scores to the result sheet");
                ui.checkbox(&mut s.scree, "Scree plot (new graph)");
                ui.horizontal(|ui| {
                    ui.label("Plots of two components: horizontal axis");
                    ui.add(egui::DragValue::new(&mut s.x_component).range(1..=1000));
                    ui.label("vertical axis");
                    ui.add(egui::DragValue::new(&mut s.y_component).range(1..=1000));
                });
                ui.checkbox(&mut s.loading_plot, "Loading plot (the loadings as vectors from the origin)");
                ui.checkbox(&mut s.score_plot, "Score plot");
                ui.checkbox(&mut s.biplot, "Biplot (scores as points, loadings as vectors on the top and right axes)");
                ui.weak(
                    "The sign of each eigenvector is chosen so that its components add up to a positive number, as \
                     Origin does. A covariance analysis also gives the test that the remaining eigenvalues are equal.",
                );
            }
            AnalysisKind::Roc => {
                let s = &mut self.roc;
                ui.label("The chosen columns are the test variables; the last one is the state column.");
                ui.horizontal(|ui| {
                    ui.label("Positive state value");
                    ui.add(egui::TextEdit::singleline(&mut s.positive).desired_width(80.0));
                });
                ui.horizontal(|ui| {
                    ui.label("Test direction");
                    ui.radio_value(&mut s.higher_is_positive, true, "Positive v.s. High");
                    ui.radio_value(&mut s.higher_is_positive, false, "Positive v.s. Low");
                });
                ui.horizontal(|ui| {
                    ui.label("Confidence level (%)");
                    ui.add(egui::DragValue::new(&mut s.confidence).speed(0.1).range(1.0..=99.99));
                });
                ui.checkbox(&mut s.curve, "ROC curve (new graph, with the diagonal)");
                ui.weak(
                    "The area under the curve with Origin's standard error and confidence interval, the                      curve points with their cut-offs, and the cut-off with the largest sensitivity + specificity - 1.",
                );
            }
            AnalysisKind::QuantReg => {
                ui.label("The chosen columns are the independent variables; the last one is the dependent variable.");
                let mut text = self.quantiles_text();
                ui.horizontal(|ui| {
                    ui.label("Quantiles");
                    ui.add(egui::TextEdit::singleline(&mut text).desired_width(200.0))
                        .on_hover_text("Comma separated, each between 0 and 1");
                });
                self.quantiles = Some(text);
                let s = &mut self.quantreg;
                ui.checkbox(&mut s.intercept, "Fit an intercept");
                ui.checkbox(&mut s.graph, "Graph of the data and the fitted lines (one independent column)");
                ui.weak(
                    "The exact solution of each quantile (the sum of check losses is as small as it can be); no standard errors.",
                );
            }
            AnalysisKind::Qpcr => {
                ui.label("The chosen columns are the sample group, the target Ct and the reference Ct.");
                let s = &mut self.qpcr;
                ui.horizontal(|ui| {
                    ui.label("Control group (empty: the first)");
                    ui.add(egui::TextEdit::singleline(&mut s.control).desired_width(100.0));
                });
                ui.horizontal(|ui| {
                    ui.label("Amplification factor: target");
                    ui.add(egui::DragValue::new(&mut s.e_target).speed(0.01).range(1.0..=3.0));
                    ui.label("reference");
                    ui.add(egui::DragValue::new(&mut s.e_reference).speed(0.01).range(1.0..=3.0));
                });
                ui.horizontal(|ui| {
                    ui.label("Confidence level");
                    let mut pct = s.confidence * 100.0;
                    if ui.add(egui::DragValue::new(&mut pct).speed(0.1).range(50.0..=99.99).suffix(" %")).changed() {
                        s.confidence = pct / 100.0;
                    }
                });
                ui.checkbox(&mut s.graph, "Expression ratio graph");
            }
            AnalysisKind::QpcrCurve => {
                ui.label("The chosen columns are the quantity (copies, dilution) and the Ct.");
                ui.checkbox(&mut self.qpcr_curve.graph, "Standard curve graph");
            }
            AnalysisKind::Cox => {
                ui.label("The chosen columns are the covariates, then the time and the censor column.");
                let s = &mut self.cox;
                ui.horizontal(|ui| {
                    ui.label("Censoring value");
                    ui.add(egui::TextEdit::singleline(&mut s.censor).desired_width(80.0));
                });
                ui.horizontal(|ui| {
                    ui.label("Hazard-ratio confidence");
                    let mut pct = s.confidence * 100.0;
                    if ui.add(egui::DragValue::new(&mut pct).speed(0.1).range(50.0..=99.99).suffix(" %")).changed() {
                        s.confidence = pct / 100.0;
                    }
                });
                ui.checkbox(&mut s.strata, "A strata column follows the censor column");
                ui.checkbox(&mut s.residuals, "Martingale, deviance and Schoenfeld residuals");
                ui.checkbox(&mut s.ph_test, "Test proportional hazards (Grambsch-Therneau)");
                ui.horizontal(|ui| {
                    ui.label("Also the survival at covariates");
                    ui.add(egui::TextEdit::singleline(&mut self.cox_at).hint_text("values").desired_width(120.0));
                });
                ui.horizontal(|ui| {
                    ui.label("Tied times");
                    ui.radio_value(&mut s.efron, false, "Breslow");
                    ui.radio_value(&mut s.efron, true, "Efron");
                });
                ui.checkbox(&mut s.graph, "Survival at the covariate means");
                ui.checkbox(&mut s.frequency, "Last column: frequency (count) of each row");
            }
            AnalysisKind::LifeRegression => {
                ui.label("The chosen columns are the covariates, then the time and the censor column.");
                let s = &mut self.life_reg;
                ui.horizontal(|ui| {
                    ui.label("Censoring value");
                    ui.add(egui::TextEdit::singleline(&mut s.censor).desired_width(80.0));
                });
                ui.horizontal(|ui| {
                    ui.label("Distribution");
                    for (d, name) in [
                        (LifeDist::Weibull, "Weibull"),
                        (LifeDist::Lognormal, "Lognormal"),
                        (LifeDist::Exponential, "Exponential"),
                    ] {
                        ui.radio_value(&mut s.dist, d, name);
                    }
                });
                ui.checkbox(&mut s.frequency, "Last column: frequency (count) of each row");
                ui.horizontal(|ui| {
                    ui.label("Percentile lives (%)");
                    ui.add(
                        egui::TextEdit::singleline(&mut self.life_reg_percentiles)
                            .hint_text("10 50 90")
                            .desired_width(100.0),
                    );
                    ui.label("at covariates");
                    ui.add(
                        egui::TextEdit::singleline(&mut self.life_reg_at).hint_text("their means").desired_width(100.0),
                    );
                });
                ui.horizontal(|ui| {
                    ui.label("Percentile interval level");
                    let mut pct = s.confidence * 100.0;
                    if ui.add(egui::DragValue::new(&mut pct).speed(0.1).range(50.0..=99.99).suffix(" %")).changed() {
                        s.confidence = pct / 100.0;
                    }
                });
            }
            AnalysisKind::Turnbull => {
                ui.label("The chosen columns are the start and the end of each failure window: equal for an exact failure, an empty end if it was still working, an empty start if it failed before the end.");
                let s = &mut self.turnbull;
                ui.checkbox(&mut s.graph, "Survival graph");
                ui.checkbox(&mut s.frequency, "Last column: frequency (count) of each row");
            }
            AnalysisKind::LifeDistArbitrary => {
                ui.label("The chosen columns are the start and the end of each failure window: equal for an exact failure, an empty end if it was still working, an empty start if it failed before the end.");
                let s = &mut self.life_any;
                ui.horizontal(|ui| {
                    ui.label("Distribution");
                    for (d, name) in [
                        (LifeDist::Weibull, "Weibull"),
                        (LifeDist::Lognormal, "Lognormal"),
                        (LifeDist::Exponential, "Exponential"),
                        (LifeDist::Normal, "Normal"),
                    ] {
                        ui.radio_value(&mut s.dist, d, name);
                    }
                });
                ui.checkbox(&mut s.graph, "Fitted CDF and survival graph");
                ui.checkbox(&mut s.frequency, "Last column: frequency (count) of each row");
            }
            AnalysisKind::LifeDist => {
                let s = &mut self.life;
                ui.label("The chosen columns are the time and the censor column.");
                ui.horizontal(|ui| {
                    ui.label("Censoring value");
                    ui.add(egui::TextEdit::singleline(&mut s.censor).desired_width(80.0));
                });
                ui.horizontal(|ui| {
                    ui.label("Distribution");
                    for d in [LifeDist::Weibull, LifeDist::Lognormal, LifeDist::Exponential, LifeDist::Normal] {
                        let name = match d {
                            LifeDist::Weibull => "Weibull",
                            LifeDist::Lognormal => "Lognormal",
                            LifeDist::Exponential => "Exponential",
                            LifeDist::Normal => "Normal",
                        };
                        ui.radio_value(&mut s.dist, d, name);
                    }
                });
                ui.checkbox(&mut s.graph, "Probability, density, survival and hazard graphs");
                ui.checkbox(&mut s.frequency, "Last column: frequency (count) of each row");
            }
            AnalysisKind::Stability => {
                ui.label("The chosen columns are the time, the response and, optionally, the batch.");
                let s = &mut self.stability;
                for (name, limit, start) in [
                    ("Lower specification limit", &mut s.lower, 95.0),
                    ("Upper specification limit", &mut s.upper, 105.0),
                ] {
                    ui.horizontal(|ui| {
                        let mut on = limit.is_some();
                        if ui.checkbox(&mut on, name).changed() {
                            *limit = on.then_some(start);
                        }
                        if let Some(v) = limit {
                            ui.add(egui::DragValue::new(v).speed(0.01));
                        }
                    });
                }
                ui.horizontal(|ui| {
                    ui.label("Confidence level of the bounds");
                    let mut pct = s.confidence * 100.0;
                    if ui.add(egui::DragValue::new(&mut pct).speed(0.1).range(50.0..=99.99).suffix(" %")).changed() {
                        s.confidence = pct / 100.0;
                    }
                });
                ui.horizontal(|ui| {
                    ui.label("Poolability significance level");
                    ui.add(egui::DragValue::new(&mut s.alpha).speed(0.01).range(0.01..=0.5));
                });
                ui.checkbox(&mut s.graph, "Graph of the batches, fits, bounds and limits");
            }
            AnalysisKind::Capability => {
                ui.label("Every chosen column is a process of its own, in row order; missing values are left out.");
                let s = &mut self.capability;
                for (name, limit, start) in
                    [("Lower specification limit", &mut s.lsl, 0.0), ("Upper specification limit", &mut s.usl, 1.0)]
                {
                    ui.horizontal(|ui| {
                        let mut on = limit.is_some();
                        if ui.checkbox(&mut on, name).changed() {
                            *limit = on.then_some(start);
                        }
                        if let Some(v) = limit {
                            ui.add(egui::DragValue::new(v).speed(0.01));
                        }
                    });
                }
                ui.horizontal(|ui| {
                    ui.label("Model");
                    ui.radio_value(&mut s.model, CapModel::Normal, "Normal");
                    ui.radio_value(&mut s.model, CapModel::Lognormal, "Lognormal");
                    ui.radio_value(&mut s.model, CapModel::Weibull, "Weibull");
                    ui.radio_value(&mut s.model, CapModel::BoxCox, "Box-Cox");
                });
                if s.model == CapModel::BoxCox {
                    ui.horizontal(|ui| {
                        let mut given = s.lambda.is_some();
                        if ui.checkbox(&mut given, "Lambda (else maximum likelihood)").changed() {
                            s.lambda = given.then_some(0.5);
                        }
                        if let Some(l) = &mut s.lambda {
                            ui.add(egui::DragValue::new(l).speed(0.01).range(-5.0..=5.0));
                        }
                    });
                } else {
                    s.lambda = None;
                }
                if matches!(s.model, CapModel::Normal | CapModel::BoxCox) {
                    ui.horizontal(|ui| {
                        ui.label("Confidence level of the Pp and Ppk intervals");
                        let mut pct = s.confidence * 100.0;
                        if ui.add(egui::DragValue::new(&mut pct).speed(0.1).range(50.0..=99.99).suffix(" %")).changed()
                        {
                            s.confidence = pct / 100.0;
                        }
                    });
                    ui.horizontal(|ui| {
                        let mut on = s.target.is_some();
                        if ui.checkbox(&mut on, "Target (Cpm)").changed() {
                            s.target = on.then(|| match (s.lsl, s.usl) {
                                (Some(l), Some(u)) => (l + u) / 2.0,
                                _ => 0.0,
                            });
                        }
                        if let Some(t) = &mut s.target {
                            ui.add(egui::DragValue::new(t).speed(0.01));
                        }
                    });
                }
                if matches!(s.model, CapModel::Normal | CapModel::BoxCox) {
                    ui.horizontal(|ui| {
                        ui.label("Subgroup size (1: moving ranges)");
                        ui.add(egui::DragValue::new(&mut s.subgroup).range(1..=10));
                    });
                }
                ui.checkbox(&mut s.graph, "Histogram with the fitted curve and the limits");
            }
            AnalysisKind::RareChart => {
                ui.label("Every chosen column is a chart of its own, in row order; missing values are left out.");
                let s = &mut self.rare;
                ui.horizontal(|ui| {
                    ui.radio_value(&mut s.kind, RareChartKind::G, "G chart (opportunities between events)");
                    ui.radio_value(&mut s.kind, RareChartKind::T, "T chart (time between events)");
                });
                match s.kind {
                    RareChartKind::G => {
                        ui.horizontal(|ui| {
                            ui.label("Limits");
                            ui.radio_value(&mut s.g_limits, GLimits::Probability, "Geometric probability");
                            ui.radio_value(&mut s.g_limits, GLimits::Sigma, "k sigma");
                        });
                    }
                    RareChartKind::T => {
                        ui.horizontal(|ui| {
                            ui.label("Distribution");
                            ui.radio_value(&mut s.t_distribution, TDistribution::Weibull, "Weibull");
                            ui.radio_value(&mut s.t_distribution, TDistribution::Exponential, "Exponential");
                        });
                    }
                }
                ui.horizontal(|ui| {
                    ui.label("Limits at sigma");
                    ui.add(egui::DragValue::new(&mut s.sigma).speed(0.05).range(0.5..=6.0));
                });
                ui.checkbox(&mut s.graph, "Control chart per column");
            }
            AnalysisKind::MonteCarlo => {
                ui.label("The draws go to a new sheet; the chosen column only says which book.");
                let inputs = self.mc_inputs.get_or_insert_with(|| {
                    self.monte_carlo
                        .inputs
                        .iter()
                        .map(|(n, d)| format!("{n} = {}", d.text()))
                        .collect::<Vec<_>>()
                        .join("\n")
                });
                ui.label("Inputs, one per line: name = normal(mean, sd), uniform(a, b), lognormal(mu, sigma), triangular(a, mode, b) or constant(v)");
                ui.add(egui::TextEdit::multiline(inputs).desired_rows(4).desired_width(360.0).code_editor());
                let s = &mut self.monte_carlo;
                ui.horizontal(|ui| {
                    ui.label("Output =");
                    ui.add(egui::TextEdit::singleline(&mut s.formula).desired_width(280.0).code_editor());
                });
                ui.horizontal(|ui| {
                    ui.label("Draws");
                    ui.add(egui::DragValue::new(&mut s.samples).range(1..=sheaf_core::analysis::MONTE_CARLO_MAX));
                    ui.label("Seed");
                    ui.add(egui::DragValue::new(&mut s.seed));
                });
                ui.horizontal(|ui| {
                    ui.label("Specification limits: lower");
                    ui.add(egui::TextEdit::singleline(&mut self.mc_lsl).desired_width(70.0).hint_text("none"));
                    ui.label("upper");
                    ui.add(egui::TextEdit::singleline(&mut self.mc_usl).desired_width(70.0).hint_text("none"));
                });
                // T490: move controllable inputs' centres within ranges to the best setting.
                ui.separator();
                ui.horizontal(|ui| {
                    ui.label("Optimize centres of");
                    ui.add(
                        egui::TextEdit::singleline(&mut self.mc_controls)
                            .desired_width(260.0)
                            .hint_text("none, or name = low:high, ...")
                            .code_editor(),
                    );
                });
                ui.add_enabled_ui(!self.mc_controls.trim().is_empty(), |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Goal");
                        ui.radio_value(&mut self.mc_aim, false, "Highest Cpk");
                        ui.radio_value(&mut self.mc_aim, true, "Mean at target, least spread:");
                        ui.add_enabled(
                            self.mc_aim,
                            egui::TextEdit::singleline(&mut self.mc_target).desired_width(70.0),
                        );
                    });
                });
            }
            AnalysisKind::MixtureDesign => {
                ui.label("The design goes to a new sheet; the chosen column only says which book.");
                let s = &mut self.mixture;
                ui.horizontal(|ui| {
                    ui.label("Components");
                    ui.add(egui::DragValue::new(&mut s.components).range(2..=10));
                    ui.radio_value(&mut s.kind, MixtureKind::SimplexLattice, "Simplex lattice, degree");
                    ui.add_enabled(
                        s.kind == MixtureKind::SimplexLattice,
                        egui::DragValue::new(&mut s.degree).range(1..=10),
                    );
                    ui.radio_value(&mut s.kind, MixtureKind::SimplexCentroid, "Simplex centroid");
                });
                ui.horizontal(|ui| {
                    ui.checkbox(&mut s.axial, "Axial check blends");
                    ui.checkbox(&mut s.centroid, "Overall centroid");
                    ui.label("Replicates");
                    ui.add(egui::DragValue::new(&mut s.replicates).range(1..=100));
                });
                let totals = self
                    .mixture_totals
                    .get_or_insert_with(|| s.totals.iter().map(|t| t.to_string()).collect::<Vec<_>>().join(", "));
                ui.horizontal(|ui| {
                    ui.label("Totals (one design per total)");
                    ui.add(egui::TextEdit::singleline(totals).desired_width(140.0));
                });
                let lower = self.mixture_lower.get_or_insert_with(String::new);
                ui.horizontal(|ui| {
                    ui.label("Lower bounds A, B, ... (empty: 0)");
                    ui.add(egui::TextEdit::singleline(lower).desired_width(140.0));
                });
                let s = &mut self.mixture;
                ui.horizontal(|ui| {
                    ui.checkbox(&mut s.randomize, "Randomize the run order, seed");
                    ui.add_enabled(s.randomize, egui::DragValue::new(&mut s.seed));
                });
            }
            AnalysisKind::FactorialDesign => {
                ui.label("The design goes to a new sheet; the chosen column only says which book.");
                let s = &mut self.design;
                ui.horizontal(|ui| {
                    ui.label("Factors");
                    ui.add(egui::DragValue::new(&mut s.factors).range(2..=10));
                    ui.label("Fraction p of 2^(k-p) (0: full)");
                    ui.add(egui::DragValue::new(&mut s.fraction).range(0..=4));
                });
                ui.horizontal(|ui| {
                    ui.label("Replicates");
                    ui.add(egui::DragValue::new(&mut s.replicates).range(1..=100));
                    ui.label("Centre points");
                    ui.add(egui::DragValue::new(&mut s.center_points).range(0..=100));
                });
                ui.horizontal(|ui| {
                    ui.checkbox(&mut s.randomize, "Randomize the run order, seed");
                    ui.add_enabled(s.randomize, egui::DragValue::new(&mut s.seed));
                });
            }
            AnalysisKind::SplitPlotDesign => {
                ui.label("The design goes to a new sheet; the chosen column only says which book.");
                let s = &mut self.split_design;
                ui.horizontal(|ui| {
                    ui.label("Whole-plot (hard-to-change) factors");
                    ui.add(egui::DragValue::new(&mut s.whole_plot_factors).range(1..=7));
                    ui.label("Subplot factors");
                    ui.add(egui::DragValue::new(&mut s.subplot_factors).range(1..=7));
                });
                ui.horizontal(|ui| {
                    ui.label("Whole-plot replicates");
                    ui.add(egui::DragValue::new(&mut s.replicates).range(1..=100));
                    ui.checkbox(&mut s.randomize, "Randomize, seed");
                    ui.add_enabled(s.randomize, egui::DragValue::new(&mut s.seed));
                });
            }
            AnalysisKind::SplitPlotEffects => {
                ui.label("The chosen columns are the whole plot, the factors (whole-plot factors first) and, last, the response.");
                let s = &mut self.split_effects;
                ui.horizontal(|ui| {
                    ui.label("Whole-plot factors (the first factor columns)");
                    ui.add(egui::DragValue::new(&mut s.whole_plot_factors).range(1..=7));
                });
                ui.checkbox(&mut s.graph, "Effects graph");
            }
            AnalysisKind::FactorialEffects => {
                ui.label("The chosen columns are the factors (two levels each) and, last, the response.");
                ui.checkbox(&mut self.factorial.graph, "Effects graph with Lenth's margin of error");
            }
            AnalysisKind::RandomForest => {
                ui.label("The chosen columns are the predictors (numbers) and, last, the response.");
                let s = &mut self.forest;
                ui.horizontal(|ui| {
                    ui.label("Task");
                    ui.radio_value(&mut s.task, ForestTask::Auto, "Auto (classes for text)");
                    ui.radio_value(&mut s.task, ForestTask::Classification, "Classification");
                    ui.radio_value(&mut s.task, ForestTask::Regression, "Regression");
                });
                ui.horizontal(|ui| {
                    ui.label("Trees");
                    ui.add(egui::DragValue::new(&mut s.trees).range(1..=10_000));
                    ui.label("Predictors per split (0: auto)");
                    ui.add(egui::DragValue::new(&mut s.mtry).range(0..=1000));
                });
                ui.horizontal(|ui| {
                    ui.label("Smallest leaf (0: auto)");
                    ui.add(egui::DragValue::new(&mut s.min_leaf).range(0..=1000));
                    ui.label("Depth limit (0: none)");
                    ui.add(egui::DragValue::new(&mut s.max_depth).range(0..=1000));
                    ui.label("Seed");
                    ui.add(egui::DragValue::new(&mut s.seed));
                });
                ui.checkbox(&mut s.graph, "Importance graph");
            }
            AnalysisKind::Arima => {
                ui.label("Every chosen column is a series of its own, in row order; it must have no missing values.");
                let s = &mut self.arima;
                ui.horizontal(|ui| {
                    ui.label("AR order p");
                    ui.add(egui::DragValue::new(&mut s.p).range(0..=20));
                    ui.label("Differences d");
                    ui.add(egui::DragValue::new(&mut s.d).range(0..=3));
                    ui.label("MA order q");
                    ui.add(egui::DragValue::new(&mut s.q).range(0..=20));
                });
                ui.horizontal(|ui| {
                    let mut auto = s.auto.is_some();
                    if ui.checkbox(&mut auto, "Choose p and q by AIC, up to").changed() {
                        s.auto = auto.then_some((3, 3));
                    }
                    if let Some((mp, mq)) = &mut s.auto {
                        ui.label("p");
                        ui.add(egui::DragValue::new(mp).range(0..=10));
                        ui.label("q");
                        ui.add(egui::DragValue::new(mq).range(0..=10));
                    }
                });
                ui.horizontal(|ui| {
                    ui.label("Seasonal P");
                    ui.add(egui::DragValue::new(&mut s.seasonal.p).range(0..=5));
                    ui.label("D");
                    ui.add(egui::DragValue::new(&mut s.seasonal.d).range(0..=2));
                    ui.label("Q");
                    ui.add(egui::DragValue::new(&mut s.seasonal.q).range(0..=5));
                    ui.label("period");
                    ui.add(egui::DragValue::new(&mut s.seasonal.period).range(0..=10_000));
                });
                ui.checkbox(&mut s.constant, "Constant (mean of the differenced series)");
                ui.horizontal(|ui| {
                    ui.label("Steps to forecast");
                    ui.add(egui::DragValue::new(&mut s.horizon).range(0..=100_000));
                    ui.label("Band confidence");
                    let mut pct = s.confidence * 100.0;
                    if ui.add(egui::DragValue::new(&mut pct).speed(0.1).range(50.0..=99.99).suffix(" %")).changed() {
                        s.confidence = pct / 100.0;
                    }
                });
                ui.checkbox(&mut s.graph, "Graph of the data, predictions, forecasts and bands");
            }
            AnalysisKind::ExpSmooth => {
                ui.label("Every chosen column is a series of its own, in row order; it must have no missing values.");
                let s = &mut self.expsmooth;
                ui.horizontal(|ui| {
                    ui.label("Model");
                    ui.radio_value(&mut s.model, SmoothModel::Simple, "Simple");
                    ui.radio_value(&mut s.model, SmoothModel::Holt, "Holt (trend)");
                    ui.radio_value(&mut s.model, SmoothModel::WintersAdditive, "Holt-Winters additive");
                    ui.radio_value(&mut s.model, SmoothModel::WintersMultiplicative, "Holt-Winters multiplicative");
                });
                if matches!(s.model, SmoothModel::WintersAdditive | SmoothModel::WintersMultiplicative) {
                    ui.horizontal(|ui| {
                        ui.label("Period (points per season)");
                        ui.add(egui::DragValue::new(&mut s.period).range(2..=10_000));
                    });
                }
                let mut fixed = s.constants.is_some();
                if ui.checkbox(&mut fixed, "Fixed smoothing constants (otherwise fitted)").changed() {
                    s.constants = fixed.then_some([0.3, 0.1, 0.1]);
                }
                if let Some(c) = &mut s.constants {
                    ui.horizontal(|ui| {
                        for (i, name) in ["Alpha", "Beta", "Gamma"].iter().enumerate().take(s.model.constants()) {
                            ui.label(*name);
                            ui.add(egui::DragValue::new(&mut c[i]).speed(0.01).range(0.0..=1.0));
                        }
                    });
                }
                ui.horizontal(|ui| {
                    ui.label("Steps to forecast");
                    ui.add(egui::DragValue::new(&mut s.horizon).range(1..=100_000));
                });
                if s.model == SmoothModel::Simple {
                    s.damping = Damping::None;
                } else {
                    ui.horizontal(|ui| {
                        let mut damped = s.damping != Damping::None;
                        if ui.checkbox(&mut damped, "Damped trend").changed() {
                            s.damping = if damped { Damping::Fitted } else { Damping::None };
                        }
                        if damped {
                            let mut given = matches!(s.damping, Damping::Given(_));
                            if ui.checkbox(&mut given, "Damping factor (else fitted)").changed() {
                                s.damping = if given { Damping::Given(0.9) } else { Damping::Fitted };
                            }
                            if let Damping::Given(p) = &mut s.damping {
                                ui.add(egui::DragValue::new(p).speed(0.01).range(0.01..=1.0));
                            }
                        }
                    });
                }
                ui.horizontal(|ui| {
                    ui.label("Prediction interval level");
                    let mut pct = s.confidence * 100.0;
                    if ui.add(egui::DragValue::new(&mut pct).speed(0.1).range(50.0..=99.99).suffix(" %")).changed() {
                        s.confidence = pct / 100.0;
                    }
                });
                ui.checkbox(&mut s.graph, "Graph of the data, one-step forecasts and forecasts");
            }
            AnalysisKind::Stationarity => {
                ui.label("Every chosen column is a series of its own, in row order; it must have no missing values.");
                let s = &mut self.stationarity;
                ui.horizontal(|ui| {
                    ui.label("ADF terms");
                    ui.radio_value(&mut s.adf_terms, AdfTerms::Constant, "Constant");
                    ui.radio_value(&mut s.adf_terms, AdfTerms::Trend, "Constant and trend");
                    ui.radio_value(&mut s.adf_terms, AdfTerms::None, "None");
                });
                ui.horizontal(|ui| {
                    let mut auto = s.adf_lags.is_none();
                    if ui.checkbox(&mut auto, "ADF lags by AIC").changed() {
                        s.adf_lags = if auto { None } else { Some(1) };
                    }
                    if let Some(l) = &mut s.adf_lags {
                        ui.add(egui::DragValue::new(l).range(0..=1000));
                    }
                });
                ui.horizontal(|ui| {
                    ui.label("KPSS null hypothesis");
                    ui.radio_value(&mut s.kpss_null, KpssNull::Level, "Level stationary");
                    ui.radio_value(&mut s.kpss_null, KpssNull::Trend, "Trend stationary");
                });
                ui.horizontal(|ui| {
                    let mut auto = s.kpss_lags.is_none();
                    if ui.checkbox(&mut auto, "KPSS lags from the length").changed() {
                        s.kpss_lags = if auto { None } else { Some(4) };
                    }
                    if let Some(l) = &mut s.kpss_lags {
                        ui.add(egui::DragValue::new(l).range(0..=1000));
                    }
                });
            }
            AnalysisKind::Decompose => {
                ui.label("Every chosen column is a series of its own, in row order; it must have no missing values.");
                let s = &mut self.decompose;
                ui.horizontal(|ui| {
                    ui.label("Period (points per season)");
                    ui.add(egui::DragValue::new(&mut s.period).range(2..=10_000));
                });
                ui.horizontal(|ui| {
                    ui.label("Model");
                    ui.radio_value(&mut s.model, DecomposeModel::Additive, "Additive");
                    ui.radio_value(&mut s.model, DecomposeModel::Multiplicative, "Multiplicative");
                });
                ui.checkbox(&mut s.graph, "Data with trend, seasonal and residual graphs");
            }
            AnalysisKind::Acf => {
                ui.label("Every chosen column is a series of its own, in row order; missing values are left out.");
                let s = &mut self.acf;
                ui.horizontal(|ui| {
                    let mut auto = s.lags == 0;
                    if ui.checkbox(&mut auto, "Lags chosen from the length (10 log10 n)").changed() {
                        s.lags = if auto { 0 } else { 20 };
                    }
                    if !auto {
                        ui.label("Largest lag");
                        ui.add(egui::DragValue::new(&mut s.lags).range(1..=10_000));
                    }
                });
                ui.horizontal(|ui| {
                    ui.label("Confidence level of the bounds");
                    let mut pct = s.confidence * 100.0;
                    if ui.add(egui::DragValue::new(&mut pct).speed(0.1).range(50.0..=99.99).suffix(" %")).changed() {
                        s.confidence = pct / 100.0;
                    }
                });
                ui.checkbox(&mut s.graph, "ACF and PACF graphs");
            }
            AnalysisKind::Manhattan => {
                ui.label("The chosen columns are the group (chromosome), the position and the p-value, in that order.");
                let s = &mut self.manhattan;
                ui.horizontal(|ui| {
                    ui.label("Significance line at p =");
                    ui.add(egui::DragValue::new(&mut s.threshold).speed(1e-9).range(1e-300..=0.999_999));
                });
                ui.checkbox(&mut s.graph, "Manhattan graph (one scatter per group and the line)");
            }
            AnalysisKind::TimePivot => {
                ui.label(
                    "Every chosen column is summarized per period of its X column, which holds dates or date-times.",
                );
                let s = &mut self.timepivot;
                ui.horizontal(|ui| {
                    ui.label("Period");
                    egui::ComboBox::from_id_salt("timepivot_period").selected_text(s.period.name()).show_ui(ui, |ui| {
                        for p in PivotPeriod::ALL {
                            ui.selectable_value(&mut s.period, p, p.name());
                        }
                    });
                    ui.label("Statistic");
                    egui::ComboBox::from_id_salt("timepivot_stat").selected_text(s.stat.name()).show_ui(ui, |ui| {
                        for st in PivotStat::ALL {
                            ui.selectable_value(&mut s.stat, st, st.name());
                        }
                    });
                });
                // Custom intervals and fiscal years (T404).
                ui.horizontal(|ui| {
                    let label = ui.label("Every");
                    let mut n = s.n();
                    ui.add(egui::DragValue::new(&mut n).range(1..=1000)).labelled_by(label.id);
                    s.every = n;
                    ui.label(format!("{}(s)", s.period.name().to_lowercase()));
                });
                const MONTHS: [&str; 12] =
                    ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
                let fiscal = matches!(s.period, PivotPeriod::Month | PivotPeriod::Quarter | PivotPeriod::Year);
                ui.add_enabled_ui(fiscal, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Fiscal year starts in");
                        let mut m = s.first_month();
                        egui::ComboBox::from_id_salt("timepivot_fiscal").selected_text(MONTHS[m as usize - 1]).show_ui(
                            ui,
                            |ui| {
                                for (i, name) in MONTHS.iter().enumerate() {
                                    ui.selectable_value(&mut m, i as u32 + 1, *name);
                                }
                            },
                        );
                        s.fiscal_start = m;
                    });
                });
                // A second grouping (T420).
                ui.horizontal(|ui| {
                    ui.label("Within each period, by");
                    let shown = s.by.map_or("(none)", |b| b.name());
                    egui::ComboBox::from_id_salt("timepivot_by").selected_text(shown).show_ui(ui, |ui| {
                        ui.selectable_value(&mut s.by, None, "(none)");
                        for b in PivotBy::ALL {
                            ui.selectable_value(&mut s.by, Some(b), b.name());
                        }
                    });
                });
                ui.weak(
                    "Weeks start on Monday; minutes and hours count from midnight, days and weeks from the first day \
                     of the data; a fiscal year is named by the year it ends in; periods without a value are left out.",
                );
            }
            AnalysisKind::BinScatter => {
                ui.label("Every chosen column is binned against its X column (or the row number).");
                let s = &mut self.binscatter;
                let (mut n, mut width) = match s.rule {
                    BinRule::EqualWidth(n) => (n, true),
                    BinRule::EqualCount(n) => (n, false),
                };
                ui.horizontal(|ui| {
                    ui.label("Number of bins");
                    ui.add(egui::DragValue::new(&mut n).range(1..=100_000));
                });
                ui.horizontal(|ui| {
                    ui.label("Bins of");
                    ui.radio_value(&mut width, false, "Equal counts");
                    ui.radio_value(&mut width, true, "Equal width");
                });
                s.rule = if width { BinRule::EqualWidth(n) } else { BinRule::EqualCount(n) };
                ui.checkbox(&mut s.graph, "Graph of the data with the bin means");
            }
            AnalysisKind::ChangePoint => {
                ui.label("Every chosen column is a series of its own, in row order.");
                let s = &mut self.changepoint;
                ui.horizontal(|ui| {
                    ui.label("Change in");
                    ui.radio_value(&mut s.kind, ChangeKind::Mean, "Mean");
                    ui.radio_value(&mut s.kind, ChangeKind::Variance, "Variance");
                    ui.radio_value(&mut s.kind, ChangeKind::MeanVariance, "Mean and Variance");
                });
                let manual = self.manual_penalty.get_or_insert(10.0);
                ui.horizontal(|ui| {
                    ui.label("Penalty");
                    let name = match s.penalty {
                        ChangePenalty::Mbic => "MBIC",
                        ChangePenalty::Bic => "BIC (SIC)",
                        ChangePenalty::Aic => "AIC",
                        ChangePenalty::HannanQuinn => "Hannan-Quinn",
                        ChangePenalty::Manual(_) => "Manual",
                    };
                    egui::ComboBox::from_id_salt("changepoint_penalty").selected_text(name).show_ui(ui, |ui| {
                        ui.selectable_value(&mut s.penalty, ChangePenalty::Mbic, "MBIC");
                        ui.selectable_value(&mut s.penalty, ChangePenalty::Bic, "BIC (SIC)");
                        ui.selectable_value(&mut s.penalty, ChangePenalty::Aic, "AIC");
                        ui.selectable_value(&mut s.penalty, ChangePenalty::HannanQuinn, "Hannan-Quinn");
                        if ui.selectable_label(matches!(s.penalty, ChangePenalty::Manual(_)), "Manual").clicked() {
                            s.penalty = ChangePenalty::Manual(*manual);
                        }
                    });
                    if let ChangePenalty::Manual(v) = &mut s.penalty {
                        ui.add(egui::DragValue::new(v).speed(0.5).range(0.0..=1e6));
                        *manual = *v;
                    }
                });
                ui.horizontal(|ui| {
                    ui.label("Minimum segment length");
                    ui.add(egui::DragValue::new(&mut s.min_segment).range(1..=100_000));
                });
                ui.checkbox(&mut s.graph, "Graph of the data with the segment means");
                ui.weak("The exact optimum of normal costs plus the penalty per change (PELT).");
            }
            AnalysisKind::Kmeans => {
                let s = &mut self.kmeans;
                ui.label("Every chosen column is a variable, every row a case.");
                ui.horizontal(|ui| {
                    ui.label("Number of clusters");
                    ui.add(egui::DragValue::new(&mut s.clusters).range(2..=1000));
                });
                ui.horizontal(|ui| {
                    ui.label("Maximum passes");
                    ui.add(egui::DragValue::new(&mut s.max_passes).range(1..=10000));
                });
                ui.checkbox(&mut s.anova, "Analysis of variance of the variables by cluster");
                ui.checkbox(&mut s.membership, "Write the cluster of every row");
                ui.add_enabled(
                    s.membership,
                    egui::Checkbox::new(&mut s.member_distance, "... and its distance to the centre"),
                );
                ui.checkbox(&mut s.graph, "Scatter graph of the first two variables, one plot per cluster");
                ui.weak(
                    "The initial centres are picked from the rows by replacement in sheet order, as Origin does, so the \
                     cluster numbers depend on the order of the rows. Distances are Euclidean on the values as they are.",
                );
            }
            AnalysisKind::KaplanMeier => {
                let s = &mut self.km;
                ui.label("The chosen columns are the time, the censor column and, optionally, a grouping column.");
                ui.horizontal(|ui| {
                    ui.label("Censoring value");
                    ui.add(egui::TextEdit::singleline(&mut s.censor).desired_width(80.0));
                });
                ui.horizontal(|ui| {
                    ui.label("Confidence level (%)");
                    ui.add(egui::DragValue::new(&mut s.confidence).speed(0.1).range(1.0..=99.99));
                });
                ui.checkbox(&mut s.quartiles, "Quartile estimates");
                ui.checkbox(&mut s.mean, "Mean estimate");
                ui.label("Tests of groups");
                ui.checkbox(&mut s.log_rank, "Log-rank");
                ui.checkbox(&mut s.breslow, "Breslow");
                ui.checkbox(&mut s.tarone_ware, "Tarone-Ware");
                ui.checkbox(&mut s.curve, "Survival curves (new graph, censored cases marked)");
                ui.add_enabled(s.curve, egui::Checkbox::new(&mut s.band, "Add confidence limits to the graph"));
                ui.checkbox(&mut s.frequency, "Last column: frequency (count) of each row");
                ui.weak(
                    "Product-limit estimate with Greenwood standard errors. Quartile limits come from the linear                      limits of the curve; the mean is cut off at the largest time.",
                );
            }
            _ => unreachable!("only the multivariate tools come here"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sheaf_core::analysis::{McControl, McGoal};

    #[test]
    fn the_monte_carlo_window_reads_controls_and_the_goal() {
        let mut o = MultiOptions::default();
        let spec = |o: &MultiOptions| match o.tool(AnalysisKind::MonteCarlo) {
            Ok(Tool::MonteCarlo(s)) => Ok(s),
            Ok(_) => unreachable!(),
            Err(e) => Err(e),
        };
        assert!(spec(&o).unwrap().optimize.is_empty(), "no optimisation to start with");
        o.mc_controls = "a = 9:11, b = 1.5:2.5".into();
        let s = spec(&o).unwrap();
        assert_eq!(s.optimize[1], McControl { name: "b".into(), low: 1.5, high: 2.5 });
        assert_eq!(s.goal, McGoal::Capability);
        o.mc_aim = true;
        assert!(spec(&o).unwrap_err().contains("target"));
        o.mc_target = "20".into();
        assert_eq!(spec(&o).unwrap().goal, McGoal::Target(20.0));
        o.mc_controls = "a = 9".into();
        assert!(spec(&o).unwrap_err().contains("low:high"));
    }
}
