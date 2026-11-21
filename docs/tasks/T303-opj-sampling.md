# T303: Sampling intervals in .opj files

**Status:** done 2026-11-21 (Claude Windows "claude:3"). feature-parity "Sampling interval" (P0,
gap "not read from Origin's .opj"). Found by byte diffs of Origin 2016 projects
(`re-workspace/scratch/t303`, local): the first X and the step are f64s at `0x1D2`/`0x1DA` of the column
record and `0x25`/`0x2D` of the data-set header, and the column belongs to no X column (`0x23` and the
header's `0x52` are 0; `0x33` is 0xFFFF beside an X column). The `.opj` reader now gives such a column
its `Column::sampling`; the writer writes the interval instead of warning (only the interval's own X
long name and units are still named in the warnings), and Origin read the values back from a file
Sheaf wrote. Interpolate's results (T302) keep their grid through an `.opj` save.
**Size:** S. **Spec:** `docs/spec/opj-format.md` 4.3.

## Tests

- `crates/io/src/opj/write/feature_tests.rs`: `a_sampling_interval_is_written_where_origin_keeps_it_and_read_back`.
- `crates/io/src/opj/tests.rs`: `local_origin_files_give_the_sampling_intervals` (Origin's files, local).

## Left

- Byte `0x68` of the layer header (0 to 1 when a column got an interval) is not written; Origin read
  the interval without it.
