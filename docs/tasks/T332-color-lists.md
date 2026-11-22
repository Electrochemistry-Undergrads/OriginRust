# T332: Origin's named colour lists

**Status:** done 2026-11-21 (Claude Windows "claude:b284", graphing 1.5). The parity row "Colour palettes" said Sheaf
did not have Origin's decoded default list and no named lists. The default list was already Origin's measured one
(the base palette in order for a new graph, Q02 Basic for the area templates). This adds Origin 2016's 18 named lists,
decoded from the `Themes\Color` files of the installation (Q02 Basic decodes to the measured theme list exactly):
Plot Details > Group > Line Color > Details chooses one and recolours the plots, the project keeps its name
(`Palette::list`), and `sheaf plot --color-list NAME` (or its code, `Q03`) does the same.
**Size:** S. **Spec:** graphing 1.5.

## Left

- Choosing a list in Origin's own dialog was not measured (`themeApply2g` with a colour theme changed nothing).
- User-made named lists; writing the chosen list to .opj.

## Tests

- `crates/core/src/template.rs`: `origins_named_colour_lists_are_found_by_name_or_code`, `a_palette_names_its_list_only_when_one_was_chosen`.
- `crates/cli/tests/plot_options.rs`: `a_named_colour_list_colours_the_curves_and_an_unknown_name_lists_the_choices`.
- `crates/app/src/app/ui_tests.rs`: `the_group_tab_chooses_one_of_origins_named_colour_lists`.
