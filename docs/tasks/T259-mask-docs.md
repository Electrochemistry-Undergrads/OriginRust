# T259: Masking parity row corrected

**Status:** done 2026-11-20 (Claude Windows "claude:3"). Docs only. The parity row "Masking" said the
`.opj` reader skips masks and the writer leaves them to the companion; both read and write the
masked cells in Origin's mask block since T10 (`opj-format.md` 3.2, checked by Origin opening Sheaf's
file). The row now names what is really left: the mask colour and the Disable Masking switch. A stray
stale doc line in the `.opj` writer (on the Group/Subject counter) was removed.
**Size:** S.
