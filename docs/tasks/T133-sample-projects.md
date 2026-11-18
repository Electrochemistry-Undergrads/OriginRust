# T133: Open Sample Projects

**Status:** done 2026-11-18 (Claude Windows "t30-origin", gui-look "Sample Projects"). Origin ships its own
sample `.opju` projects; Sheaf builds its own samples from made-up data (no owner data, nothing copied
from Origin). Left: more samples (fitting, matrices, 3D once those exist).
**Size:** S. **Source:** greyed inventory package B12 (File > Open Sample Projects, Help > Open Sample
Projects > Sample Projects).
**Spec:** `docs/spec/gui-look.md` ("Sample Projects").

## Done when

Both menu items open the sample list, a sample replaces the project after the unsaved-changes question,
the inventory no longer lists the items, and fmt, strict Clippy and the full tests pass.
