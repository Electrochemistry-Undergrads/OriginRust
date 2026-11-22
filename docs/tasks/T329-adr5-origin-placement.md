# T329: Where analysis results go: ADR 0005 and Origin's placement (for the owner)

**Status:** done 2026-11-21 (Claude Windows "claude:3"), docs only: an addendum to ADR 0005 records
that T300, T302, T321, T322 and T326 put results at the end of the input sheet as measured in Origin.
**Open decision for the owner:** whether the one-value-per-row tools (Smooth, Differentiate, the
running integral, Subtract Baseline, ...) should follow Origin too.
**Size:** S (docs). **Spec:** `docs/adr/0005-analysis-results.md` (addendum 2026-11-21).

## What Origin 2016 does (measured 2026-11-21, `re-workspace/scratch/t327/o.lines.txt`, local)

On a sheet A(X), B, C: `smooth method:=0 npts:=3` on B added D "Smoothed Y1", comment "3 pts AAv smooth
of B"; `differentiate` added E "Derivative Y1", "1st derivative of B"; `integ1 oy:=<new>` added F
"Integrated X1" (X) and G "Integrated Y1", comment "Integrate from 0 to 5" CR LF "Area = 42.5"; a second
`smooth method:=1` then added H "Smoothed X1" (X) before I "Smoothed Y2" ("3 pts SG smooth of B"),
because the nearest X to the left of the end was now F.

## What Sheaf does

Point 3 of ADR 0005: the new Y column goes directly after its source ("Smoothed Current"), which keeps
the X pairing right without extra X columns. Origin's way gives numbered names, comments with the method
and an X copy where needed. Changing it would touch many tools, their tests and the CLI/MCP output, so
it was not done without the owner's decision.
