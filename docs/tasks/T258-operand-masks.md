# T258: Masked reference and operand cells in curve and column math

**Status:** done 2026-11-20 (Claude Windows "claude:3"). Bug fix (T255 open item). Simple Curve Math's
reference curve and Simple Column Math's operand column were read raw, so masked cells and rows the
worksheet filter hides still counted. They are now read with `Worksheet::values_of`: a masked
reference point is left out of the reference curve (which is interpolated without it), and a masked
operand cell gives a missing result in its row. The transform tools' second inputs are unchanged
(see T255).
**Size:** S.

## Tests

- `crates/core/src/analysis.rs`: `masked_reference_and_operand_cells_are_left_out`.
