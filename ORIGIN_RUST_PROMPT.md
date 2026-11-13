# Origin in Rust: build prompt for Claude Fable 5.2

> **For the person running this (the model can skip this box).**
>
> - Open Claude Code in `<repository checkout>`, choose **Claude Fable 5.2**, and set effort to **High**. Move to xhigh only if a hard part stalls at High.
> - Use Auto mode, or allow `git`, `gh`, `cargo`, `node` and PowerShell commands, so the model can work long stretches without waiting on approvals.
> - **First run:** send `Read ORIGIN_RUST_PROMPT.md and follow it.`
> - **Resume after running out of usage:** start a new session in the same folder and send `Read ORIGIN_RUST_PROMPT.md, then HANDOFF.md, and continue.`
> - **Resume on another computer:** run `gh repo clone OriginRust`, open Claude Code in that folder, and send the same resume line. Origin-dependent steps (REA analysis, reference outputs) only work on the machine that has Origin installed.
> - Edit the **Project settings** section if you want a different repo name or different priorities.
>
> Everything below the line is the prompt.

---

## What I want and why

I use OriginLab Origin every day for lab data. I import instrument exports, organize them in workbooks, make publication figures, and run analysis such as curve fitting, peak analysis, integration and smoothing. My copy is an old version licensed to one Windows machine. I want an application that I own and can extend: a fast native Rust desktop app that does the jobs I use Origin for, with a familiar workflow, and that can open my existing Origin project files.

Please build that app. It is a long project that will span many sessions, on different days and maybe different computers, and my Claude usage can run out in the middle of any session. So the GitHub repository is the project's memory. Anyone, including you in a fresh session, must be able to continue exactly where work stopped by reading what is pushed.

Use the REA skill (`reverse-engineer-anything`) and its MCP tools to study the installed Origin, so that the specification comes from evidence about how Origin actually behaves and not from guesses.

This codebase is yours: what it's for and how it's built are your call, and you'll keep building on it.

## Project settings (edit freely)

- **Repo root:** `<repository checkout>` (this folder).
- **GitHub:** a **private** repo named `OriginRust` on the account that `gh` is logged in to (`gh auth status` shows it). Create it on the first run if it does not exist.
- **App name:** choose a short, neutral working name that does not use "Origin" or other OriginLab trademarks. Record it in `README.md`.
- **Platform:** Windows 11 x64 first. Keep the crates that have no GUI portable.
- **My daily workflow, which sets priorities:**
  - CH Instruments (CHI) text exports: cyclic voltammetry with several cycles that must be split into separate scans, and chronoamperometry (current vs. time).
  - CSV exports from other instruments, for example ICP-OES results.
  - Overlay plots of scans, integration of peaks and of charge, and publication figures.
  - My preferred CV figure style: smooth lines without markers, no gridlines, ticks inside, a full black box around the plot, and a y axis that is symmetric about zero current. This is a good first built-in figure template.

## What is on this machine

- **Origin** in a locally configured installation folder (`ORIGIN_HOME`).
  - The main executable is the installed Origin executable. Most of the core logic is in `ok9_64.dll` (about 27 MB). Other important modules are `okUtil9_64.dll`, `ou9_64.dll`, `od9_64.dll`, `ogrid9_64.dll` (worksheet grid), `ocMath9_64.dll`, `ONAG9_64.dll` (NAG numerical library), `OK3DGL9_64.dll` (3D graphs) and `OCompiler9_64.dll` (Origin C compiler).
  - Plain-text assets that you can read directly, without REA: `FitFunc\*.FDF` (239 fit-function definitions), `Filters\*.oif` (import filters), `X-Functions\**\*.oxf`, `*.ogs` (LabTalk scripts), and `OriginC\` (Origin C source that OriginLab ships). Graph templates (`*.otp`) and worksheet templates (`*.otw`) are binary.
  - `Samples\` holds example projects (`*.opj`) and data, grouped by topic (Curve Fitting, Graphing, Import and Export, Signal Processing, Spectroscopy, and others). `Samples\COM Server and Client` shows that Origin can be driven through COM automation and LabTalk, which is useful for generating reference outputs.
- **My Origin user folder (a copy):** `OriginLab\User Files` in the repo root. It holds settings, `Origin.ini`, custom buttons and NLSF settings. Treat it as read-only.
- **REA 6.8.0:** cloned at `rea\`. The MCP server `rea` is registered globally in Claude Code, and the skill is installed as `reverse-engineer-anything` (`~\.claude\skills\reverse-engineer-anything`). The native-analysis provider is Ghidra 12.1.4 at `<local Ghidra installation>`, with Java 21 (REA's MCP entry sets `JAVA_HOME`). Hopper is not available on Windows.
- **Toolchain:** Rust 1.99.1 with cargo, Node 22, git 2.32, and gh 2.103 (logged in).

## Ground rules: clean-room, interoperable, mine

The goal is an independent program that does the same jobs, not a copy of OriginLab's program. This matters legally, because of copyright and because REA's own responsible-use terms forbid "copying proprietary software or reproducing its protected code and assets". It also matters practically, because I want code that I fully own.

- **Learn behavior, then write specs.** Use REA, the readable assets, public documentation and black-box runs of Origin to learn behavior: features and their options, defaults, file formats, how algorithms behave, and numeric results. Write what you learn as specifications in your own words under `docs/spec/`. Then implement from the spec.
- **Keep OriginLab's code and assets out of the repo and the app.** That means no decompiled pseudocode, no Origin C, LabTalk or X-Function source, and no templates, icons, bitmaps, themes, help text or sample data files. Short identifiers that interoperability needs are fine: option names, file-format field names, magic numbers, and fit-function names and formulas (formulas are math).
- **Interoperability is in scope.** Reading OPJ project files, `.FDF` fit definitions and other Origin formats so that I can import my existing work is a core goal.
- **Licensing is out of scope.** Do not analyze the `License\` folder, license checks or protection code, and do not build anything that works around them. If a question leads there, drop that route and find another one.
- **Leave the Origin installation and my Origin settings unchanged.** You may launch Origin and drive it through the GUI, COM or LabTalk scripts to produce reference outputs.
- **Respect third-party licenses.** `liborigin` (GPL-3, used by SciDAVis and LabPlot) documents much of the OPJ format. Reading it to understand the format is fine. Copying its code would force this project under the GPL, so do not do that unless I decide on that license later. The same care applies to crate licenses.
- **Raw reverse-engineering material stays local.** Ghidra projects, REA evidence bundles, decompiled excerpts and notes that quote them go only in `re-workspace\`, which git ignores. Specs in `docs/spec/` can cite REA Evidence IDs, module names and addresses, but never paste decompiled code.
- **The look is your own.** A familiar layout (menus, workbooks, graph windows, a project explorer) is fine. Draw your own icons and visual style.

## How to use REA on this target

Load the `reverse-engineer-anything` skill before your first REA call and follow it. It covers connection and recovery, summary-first results, Evidence IDs, and keeping observations, inferences and unknowns apart. Some pointers for this target:

- **Use the cheapest source first.** OriginLab's public online documentation and the plain-text assets above often answer "which options exist and what do they do" without any binary analysis. Paraphrase what you read; do not paste it.
- **Map the UI from PE resources.** The menus, dialogs and string tables in the installed Origin executable and the core DLLs are a fast map of every command and option. REA's target-free PE tools (resources, imports, exports) do not need Ghidra.
- **Use Ghidra for focused questions** that cheaper sources cannot answer, for example binary details of the OPJ format or the exact default of an algorithm. Do not sweep whole DLLs. `ok9_64.dll` is large, so expect a slow cold import, and use the skill's startup-deadline recovery if it times out.
- **Observe runtime behavior by running Origin.** REA's process capture needs Linux or macOS, so on this machine run Origin itself instead. For example, a COM or LabTalk script can import a file, run an analysis and export the results, which you then compare with your implementation.
- **Save evidence for later sessions.** At checkpoints, and before you end a long investigation, save the evidence bundle into `re-workspace\evidence\` with `reset_investigation` and `save_path`, or with `export_evidence_bundle`. List the saved bundles in `HANDOFF.md` so that a later session can import them again.

## What to build

The scope is "the jobs I use Origin for", grown in vertical slices. You own the plan. Write it as `docs/ROADMAP.md` (milestones broken into tasks with checkboxes), refine it as you learn, and keep it current. This order makes sense to me; change it when you have a reason:

1. **Inventory and foundations.** Build a feature inventory from REA and the documentation in `docs/spec/feature-inventory.md`. Tag each feature P0 (my daily workflow), P1 (common Origin use) or P2 (later). Set up the Cargo workspace, CI and architecture decision records.
2. **First vertical slice, P0 from end to end.**
   - Import a CHI CV export (split into scans) and generic CSV or TXT files through an import-options dialog.
   - Show the data in a workbook whose columns have designations (X, Y, y-error), long names, units and comments.
   - Make an overlay line plot, and edit axes, ticks, fonts, line styles and the legend.
   - Export PNG at a chosen DPI, SVG and PDF.
   - Save the project and open it again.
   - Make all of this work without a window through a CLI as well (for example `<app> plot ...`). The CLI is also how you test rendering.
3. **Analysis, part 1.** Column math with "set column values" style expressions, sort and filter, normalize, interpolate, differentiate, integrate (area under a curve, with a baseline), smoothing (Savitzky-Golay, adjacent averaging, FFT filter), baseline subtraction, peak finding and descriptive statistics.
4. **Fitting.** Linear and polynomial fits. Nonlinear least squares (Levenberg-Marquardt) with a built-in function library that covers Origin's common categories, plus user-defined functions, parameter bounds and fixed parameters. A fit report with parameters, standard errors, R², reduced chi-square and residuals. The fitted curve drawn on the graph. Import of `.FDF` files.
5. **Origin project import.** Read Origin `.opj` files: workbooks and data first, then basic graphs. OPJU can come later if I need it.
6. **Breadth.** More 2D graph types (scatter, column and bar, area, error bars, double-Y axes, multi-panel layers, log and reciprocal scales), matrices, contour plots and heatmaps, statistics (t-tests, ANOVA), batch processing and analysis templates, scripting through an embedded language (LabTalk compatibility only where it is cheap), and 3D last.

These are my architecture defaults. Change any of them through an ADR in `docs/adr/` if you find something better.

- **Crates:** roughly `core` (data model, project, undo and redo), `io` (importers, exporters, OPJ reader), `calc` (numerics), `plot` (a figure model that does not depend on the GUI, with renderers to PNG, SVG and PDF), `app` (GUI) and `cli` (no window). Rendering that does not depend on the GUI is important. Figures must look the same on screen and in exports, and it lets you test without opening a window.
- **GUI:** a native Rust stack with a spreadsheet grid that stays fast at a million rows or more, plus dockable or MDI windows. eframe/egui with egui_dock is one option; use another if an ADR justifies it.
- **Project file:** your own documented, versioned format. I lean toward something that can be inspected, for example a zipped JSON manifest with columnar data.
- **Numerics:** use established crates where they exist (for example nalgebra or faer, and rustfft), and check them against Origin's outputs.

## Proving that it works

- **Checks:** every task ends with `cargo fmt`, `cargo clippy` with no new warnings, and `cargo test` passing for the crates you touched.
- **Numeric parity:** for each analysis, produce reference results from real Origin on the same input, either by driving Origin through COM or LabTalk or by asking me to export them. Use inputs that you generated yourself or that I gave you for this purpose. Origin's numeric outputs on those inputs can be committed as test fixtures, with the tolerances written down. Tests that need files that exist only on this machine (Origin samples, my data) must skip cleanly when the files are missing, so CI still passes.
- **Visual checks:** render figures to PNG through the CLI and look at them, since you can read images. When layout fidelity matters, compare with the same figure made in Origin. Keep a small set of golden images for regression tests.
- **Independent review:** at the end of each milestone, have one fresh-context subagent review the milestone against its spec and the roadmap, then fix what it finds.
- **Test hygiene:** commit only tests that guard real behavior. Scratch scripts go in `re-workspace\scratch\` or the system temp folder.

## Checkpoints, GitHub and resuming (the most important section)

My usage can end in the middle of a session without warning, so the pushed repo must always be ready to resume. Checkpointing handles that risk. It is never a reason to cut work short, summarize early or stop.

### First-run setup

1. Write `.gitignore` before the first `git add`. It must cover at least: `target/`, `rea/` (a separate clone with its own git history), `OriginLab/` (my Origin settings), `re-workspace/`, `*.opj`, `*.opju`, `*.otp`, `*.otw`, `*.ogs`, `*.oxf`, `.env`, `*.log` and `.claude/settings.local.json`.
2. Run `git init` in the repo root if it is not a git repo yet. Add `.gitattributes` with `* text=auto`.
3. Create the private GitHub repo and push, for example with `gh repo create OriginRust --private --source . --remote origin --push`. If the repo already exists, add it as the remote instead.
4. Run `git status` and confirm that nothing from `rea/`, `OriginLab/` or `re-workspace/` is staged or pushed. This prompt file (`ORIGIN_RUST_PROMPT.md`) should be committed, so that a session on another computer has it.
5. Keep CI to one light job with caching. Private repos get a limited number of free GitHub Actions minutes, and Windows runners use them at twice the rate.

### The files that hold the project state (all committed)

- **`HANDOFF.md`** is rewritten at every checkpoint and fits on about one screen. It states:
  - the current milestone and task;
  - what was just finished, with commit hashes;
  - what is in progress and its exact state, including anything that does not compile;
  - the exact next step;
  - the commands that verify the current state;
  - open decisions and questions for me, under a "Needs from me" heading;
  - local-only resources that a resume needs, such as REA evidence bundles in `re-workspace\evidence\` and Ghidra project names.
- **`docs/ROADMAP.md`** lists milestones and tasks with checkboxes. Tick a box only after the task is verified.
- **`docs/spec/`** holds the specifications that you wrote from the reverse-engineering work.
- **`docs/adr/`** holds decisions and their reasons.
- **`docs/LESSONS.md`** is your memory between sessions. Write one entry per lesson, with a one-line summary first. Record corrections and confirmed approaches alike, with the reason each one mattered. Update an existing entry instead of adding a duplicate, and delete entries that turn out to be wrong.

### When to checkpoint

Commit and push:

- after each finished roadmap task;
- before any step that could take a long time or fail badly, such as a large Ghidra import, a big refactor or a long release build;
- after you write or change a spec or an ADR;
- in any case, before about 30 minutes or about 25 tool calls of work are unpushed. Work in progress is fine to push. Give it a `wip:` commit subject and describe its state in `HANDOFF.md`.

Each checkpoint works like this. Update `HANDOFF.md` and the `ROADMAP.md` ticks. Run `git add -A`, then read `git status` and make sure nothing that should stay local is staged. Commit with a Conventional Commits subject, then `git push`. If the push fails, keep committing locally, retry later, and note the failure in `HANDOFF.md`. When a milestone is complete, tag it (for example `m1-foundations`) and push the tag.

### Resume protocol, at the start of every session

1. If `HANDOFF.md` does not exist, this is the first session. Do the first-run setup, then start milestone 1.
2. Otherwise, read `HANDOFF.md`, `docs/ROADMAP.md`, `docs/LESSONS.md` and `git log --oneline -15`. Run `git fetch` and `git status` to confirm that the local tree matches GitHub. If the folder is missing, clone the repo.
3. Build and run the tests to confirm the state that `HANDOFF.md` describes. If the state differs, fix `HANDOFF.md` first.
4. Continue from the next step. Do not redo finished work. Trust ticks that were verified.

## How I'd like you to work

### Autonomy and approvals

I am not watching in real time and cannot answer questions mid-task, so asking "Want me to...?" or "Shall I...?" blocks the work. For reversible actions that follow from this prompt, go ahead without asking.

These actions are approved and need no confirmation:

- creating the private `OriginRust` repo, and commits, pushes, tags and releases on it;
- adding crates and rustup components;
- building, testing and running the app and the CLI;
- running any REA analysis on the Origin installation;
- launching and scripting Origin to produce reference outputs;
- creating and deleting files inside the repo and `re-workspace\`;
- retrying failed or timed-out commands that are safe to repeat.

These actions need my go-ahead first, and become fine once I give it:

- making the repo public or sharing it;
- force-pushing or rewriting pushed history;
- changing anything under `C:\Program Files\OriginLab` or in my Origin settings;
- installing system-wide software other than crates and rustup components (Ghidra and Java are already here);
- committing any of my own research data.

Anything that involves Origin's licensing stays out of scope even with a go-ahead.

### Finishing work

Please avoid stopping while you still owe work that this prompt asks for. Be persistent and do not stop unless needed. Status notes are welcome, and so are questions about open decisions, but carry on with work that does not depend on my answer. If you notice yourself inviting me to redirect you or offering to wait, move on to the next part of the task instead. You may meet ordinary obstacles like errors, timeouts, locked files, empty results or failing tools. Diagnose them first, and when they are not real blockers, work through them with the access you have: wait and retry, fix the request, or use another tool or source. If something is clearly a deliberate hard blocker, such as the licensing boundary above or access that was intentionally withheld, leave it alone, say plainly what you found, and look for another safe way to finish. When a question for me comes up, write it under "Needs from me" in `HANDOFF.md`, state the assumption you are working under, and continue. Before you end a turn, check your last paragraph. If it is a plan or a promise about work you have not done, do that work now with tool calls.

### Honest progress

Before you report progress in `HANDOFF.md`, in commit messages or to me, check each claim against a tool result from this session. Report only work that you can point to evidence for. If something is not verified yet, say so. If tests fail, say so and include the output. If a step was skipped, say that. When something is done and verified, state it plainly.

### Scope inside a task

Restructure the code freely when it makes the next tasks easier. Ideas that go beyond the roadmap go into `docs/ROADMAP.md` as proposals, not into side work. When a task is ambiguous, choose the reading that best serves my daily workflow, write the assumption down, and keep going.

### Subagents

Subagents cost tokens and my usage is limited, so delegate only when it clearly saves time. Good cases are independent reverse-engineering questions, or work on a separate crate that does not touch files you are editing. Brief each subagent fully: the goal, the files, the clean-room rules, and what it must return. Do small tasks yourself, and do not start multi-agent workflows.

### Communicating clearly

These rules apply to `HANDOFF.md`, commit messages, specs and your messages to me. Write for an intelligent reader who has little context. The reader has not seen your tool calls, your reasoning or the files you opened, and may be switching in from other work. Put the key points in the first few sentences. Use plain words and complete sentences. Do not use terms or abbreviations that you made up during the work; say what the thing is. Leave out unnecessary detail, but keep the context the reader needs to act.

### When a session ends normally

Checkpoint everything. Then write a short recap that stands on its own: what is done and verified, the current state, the next step, and anything you need from me.

## Things I can give you

Ask for these under "Needs from me" in `HANDOFF.md` and keep working while you wait:

- real CHI export files (CV and i-t);
- my typical `.opj` projects;
- the figures I make most often, as examples to match;
- Origin outputs for specific analyses that are hard to script.
