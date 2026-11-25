# T426: Faster project save

**Status:** done 2026-11-24 (Claude Windows "claude:2").
origin-2026-2027-gaps "Faster project save" (Origin 2026 reports about 40 % faster saving of large
mixed projects; a performance target for `.sheaf`). Measured: deflating the data members was most of
the save. Projects with more than 4 MB of data now deflate the members on up to 8 threads into archives
in memory, copied into the file in order (`zip::ZipWriter::merge_archive`), so the file is the same as
before. 8 books x 32 columns x 50 000 rows (76 MB): save 4.6 s -> 1.15 s; load 0.6 to 0.7 s.
**Size:** S. **Spec:** `docs/spec/project-file-format.md` (Writing rules).

## Tests

- `project_file::tests::a_large_project_saved_on_several_threads_reads_back_the_same` (round trip and
  member order), `the_members_are_split_in_order_without_gaps` (io); the timing test
  `crates/io/tests/save_speed.rs` (ignored; `cargo test -p sheaf-io --test save_speed -- --ignored --nocapture`).

## Left

- `project.json` is still pretty-printed on one thread; autosave of very large projects could use the
  same path (it calls `save`, so it already does).
