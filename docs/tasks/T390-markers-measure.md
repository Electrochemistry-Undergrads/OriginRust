# T390: Measure the graph Data menu's data markers against Origin 2016

**Status:** done 2026-11-23 (Claude Windows "claude:work", graphing 40). Origin 2016's graph Data menu and Mark Data
Range (Ctrl+Alt+M) were measured from its window. The markers sit at the first and last points as red vertical lines
across the whole layer, with a cursor on each point, and they stay after Esc. Sheaf's markers are now red lines
across the layer with a short bar at the point (its length following Analysis Markers' size), instead of black
I-beams.
**Size:** S. **Follows:** T118. **Spec:** graphing 40.

## Left

- Per plot or per layer, how Origin moves a marker, and Set Display Range's result.

## Tests

- `crates/plot/src/zoom_tests.rs`: `analysis_markers_hide_partial_and_size` (a red line across the layer, the bar).
