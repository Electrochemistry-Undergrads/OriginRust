//! Colour scales and bubble scales of a layer (T80, graphing spec 26): Origin's Graph > New Color
//! Scale and the legend's Add Color Scale / Add Bubble Scale. A scale is an object beside the layer's
//! frame that describes a plot whose symbols take their colour (colour scale) or size (bubble scale)
//! from a Z column. Sheaf links a scale to the first visible plot of the layer that is mapped that way.

use crate::graph::{Layer, Plot};
use serde::{Deserialize, Serialize};

/// Which property of the symbols the scale describes.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LayerScaleKind {
    /// The colour ramp from the smallest to the largest Z.
    Color,
    /// Sample circles from the smallest to the largest Z.
    Bubble,
}

impl LayerScaleKind {
    pub fn name(self) -> &'static str {
        match self {
            LayerScaleKind::Color => "color scale",
            LayerScaleKind::Bubble => "bubble scale",
        }
    }
}

/// A colour scale or bubble scale of a layer.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LayerScale {
    pub kind: LayerScaleKind,
    /// The title text; `None` is the name of the Z column of the plot the scale describes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
}

impl LayerScale {
    pub fn new(kind: LayerScaleKind) -> Self {
        LayerScale { kind, title: None }
    }

    /// The plot of `layer` this kind of scale describes: the first visible plot with a colour (or size)
    /// column.
    pub fn plot_of(layer: &Layer, kind: LayerScaleKind) -> Option<&Plot> {
        layer.plots.iter().find(|p| {
            p.visible
                && match kind {
                    LayerScaleKind::Color => p.color_from.is_some(),
                    LayerScaleKind::Bubble => p.size_from.is_some(),
                }
        })
    }

    /// True when `layer` has a plot a scale of this kind can describe.
    pub fn applies(layer: &Layer, kind: LayerScaleKind) -> bool {
        LayerScale::plot_of(layer, kind).is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::template::Template;

    #[test]
    fn round_trips_and_old_layers_load_without_the_field() {
        let mut p = crate::Project::new();
        let g = p.add_graph(Template::Scatter);
        let layer = &mut p.graph_mut(g).unwrap().layers[0];
        let json = serde_json::to_string(layer).unwrap();
        assert!(!json.contains("scales"));
        assert!(serde_json::from_str::<Layer>(&json).unwrap().scales.is_empty());
        let mut s = LayerScale::new(LayerScaleKind::Bubble);
        s.title = Some("Size".into());
        layer.scales.push(s);
        layer.scales.push(LayerScale::new(LayerScaleKind::Color));
        let back: Layer = serde_json::from_str(&serde_json::to_string(layer).unwrap()).unwrap();
        assert_eq!(back.scales, layer.scales);
    }

    #[test]
    fn applies_only_with_a_mapped_visible_plot() {
        let mut p = crate::Project::new();
        let g = p.add_graph(Template::Scatter);
        let layer = &mut p.graph_mut(g).unwrap().layers[0];
        assert!(!LayerScale::applies(layer, LayerScaleKind::Color));
        assert!(!LayerScale::applies(layer, LayerScaleKind::Bubble));
    }
}
