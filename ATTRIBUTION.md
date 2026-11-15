# Attribution

Sheaf's original code and project assets are licensed under MIT OR Apache-2.0; see
[LICENSE-MIT](LICENSE-MIT), [LICENSE-APACHE](LICENSE-APACHE), and [NOTICE](NOTICE).
Third-party material keeps its own license. Add provenance and license information here
when introducing an asset; do not assume the project license covers material from elsewhere.

## Project assets and fixtures

| Path | Creator and source | License |
|---|---|---|
| `docs/brand/sheaf.png` | Project owner; original logo, ownership confirmed by the owner | MIT OR Apache-2.0 |
| `crates/app/assets/icon/*` | Sheaf contributors; app icon generated from `docs/brand/sheaf.png` by `tools/make-icon.py` | MIT OR Apache-2.0 |
| `docs/images/macos-desktop.png` | Sheaf contributors; screenshot of Sheaf displaying synthetic data | MIT OR Apache-2.0 for the original screenshot; embedded third-party typefaces retain their own licenses |
| `crates/plot/assets/sRGB-v4.icc` | Compact sRGB v4 ICC profile, the same file svg2pdf 0.13 bundles as `src/icc/sRGB-v4.icc`; copied from that crate package | [CC0 1.0](https://creativecommons.org/publicdomain/zero/1.0/) (svg2pdf's NOTICE: the ICC profiles in `src/icc/*`) |
| `crates/plot/tests/golden/*.svg` | Sheaf contributors; figures generated from synthetic test data | MIT OR Apache-2.0 |
| `crates/io/tests/fixtures/sheaf-written.opj` | Sheaf contributors; project written by Sheaf, not an Origin-supplied sample | MIT OR Apache-2.0 |

Numerical reference fixtures are documented in `tools/origin-ref/` and the relevant specifications.
Those scripts are project-authored callers of Origin's scripting/automation interfaces. This
statement does not grant redistribution rights to Origin software, vendor samples, documentation,
or research data. Do not commit those materials without a separate rights review.

## Fonts embedded through dependencies

The desktop build enables eframe's `default_fonts` feature. The locked dependency
`epaint_default_fonts` 0.33.3 embeds these fonts. The license texts below are copied from that
exact crate package; the original upstream font files are not vendored in this repository.

| Font | Copyright / source | License text |
|---|---|---|
| Hack Regular | Copyright 2018 Source Foundry Authors; Copyright 2003 Bitstream, Inc.; DejaVu contributions placed in the public domain. [Hack](https://github.com/source-foundry/Hack) | [MIT and Bitstream Vera terms](third-party/egui-default-fonts/LICENSE-Hack.txt) |
| Noto Emoji Regular | Copyright 2013 Google Inc. [Noto Emoji](https://github.com/googlefonts/noto-emoji) | [SIL Open Font License 1.1](third-party/egui-default-fonts/LICENSE-NotoEmoji.txt) |
| Ubuntu Light | Copyright 2011 Canonical Ltd.; designed by Dalton Maag. [Ubuntu font](https://design.ubuntu.com/font) | [Ubuntu Font Licence 1.0](third-party/egui-default-fonts/LICENSE-Ubuntu.txt) |
| emoji-icon-font | Copyright 2014 John Slegers. [emoji-icon-font](https://github.com/jslegers/emoji-icon-font) | [MIT](third-party/egui-default-fonts/LICENSE-emoji-icon-font.txt) |

System fonts used to render or embed text in exported figures are supplied by the user's machine;
their redistribution/embedding terms must be checked separately when distributing font files.

## Binary distributions

Ship this file, the font license texts, and the applicable dependency copyright/license notices
with desktop/CLI binary packages. `Cargo.lock` identifies build inputs; it is not a substitute for
their notices. Recheck the selected features and target platform when packaging a release.

Origin and OriginPro belong to OriginLab. Compatibility references do not imply affiliation,
sponsorship, endorsement, or a license to vendor code, artwork, or branding.
