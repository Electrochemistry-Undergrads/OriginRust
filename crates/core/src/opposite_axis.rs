//! Layer Management display overrides for the edge facing a Cartesian axis.

use crate::graph::{Axis, TickDirection};
use serde::{Deserialize, Serialize};

/// An absent override preserves the appearance of an existing mirrored or paired axis.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct OppositeAxisStyle {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tick_direction: Option<TickDirection>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub show_title: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub show_tick_labels: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tick_label_formula: Option<String>,
}

impl OppositeAxisStyle {
    pub fn is_default(&self) -> bool {
        *self == Self::default()
    }
}

impl Axis {
    /// Independently styled ticks need an axis line. Old mirrors may show ticks by themselves.
    pub fn opposite_ticks_shown(&self) -> bool {
        self.opposite_ticks && (self.show_opposite || self.opposite_style.tick_direction.is_none())
    }

    /// The facing major and minor tick directions, including visibility.
    pub fn opposite_tick_directions(&self) -> [TickDirection; 2] {
        if !self.opposite_ticks_shown() {
            return [TickDirection::None; 2];
        }
        self.opposite_style.tick_direction.map_or([self.major_tick, self.minor_tick], |direction| [direction; 2])
    }

    pub fn opposite_title_visible(&self) -> bool {
        self.opposite_style.show_title.unwrap_or_else(|| self.has_paired_mirror() && self.show_title)
    }

    pub fn opposite_labels_visible(&self) -> bool {
        self.opposite_style.show_tick_labels.unwrap_or_else(|| self.has_paired_mirror() && self.show_tick_labels)
    }

    pub fn opposite_formula(&self) -> &str {
        self.opposite_style.tick_label_formula.as_deref().unwrap_or(&self.tick_label_formula)
    }

    fn has_paired_mirror(&self) -> bool {
        self.opposite_offset_pct.is_some() && (self.show_opposite || self.opposite_ticks)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{WindowId, template::Template};

    #[test]
    fn loading_a_legacy_paired_axis_keeps_its_inherited_styles() {
        let mut graph = Template::Line.new_graph(WindowId(1), "G");
        let axis = &mut graph.layers[0].x_axis;
        axis.opposite_ticks = true;
        axis.show_opposite = true;
        axis.opposite_offset_pct = Some(12.0);
        axis.tick_label_formula = "x*2".into();
        let saved = serde_json::to_value(&*axis).unwrap();
        assert!(saved.get("opposite_style").is_none());
        let axis: Axis = serde_json::from_value(saved).unwrap();
        assert!(axis.opposite_labels_visible() && axis.opposite_title_visible());
        assert_eq!(axis.opposite_tick_directions(), [axis.major_tick, axis.minor_tick]);
        assert_eq!(axis.opposite_formula(), "x*2");
    }
}
