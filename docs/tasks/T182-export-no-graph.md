# T182: Export graphs from a workbook or with no window (T63 group E)

**Status:** done 2026-11-19 (Claude Windows "acct1"), measured in Origin 2016 on 2026-11-24.
File > Export > Graphs..., Graph as Image... and Multi-Page PDF... outside a graph window open
Export Graphs on every project graph. The image commands select PNG and the PDF command ticks
the multi-page box.
**Size:** S. **Source:** greyed inventory brief T63, group E.
**Spec:** `docs/spec/graphing.md` 9.7a.

## Origin 2016 readback

The workbook entries As Image File... and As Multipage PDF File... opened the current sheet in
`expWks` and `expPDFw`. Export Graphs opened the active graph in `expGraph`. The graph command
opened no dialog when queued from a workbook, and calling `expGraph` directly reported an invalid
active graph. Section 9.7a records the native probe and Sheaf's project graph shortcuts.
The File menu with every project window hidden remains unverified after Origin COM startup failures.
