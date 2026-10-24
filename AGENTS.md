# ReGTA CTW PC Recomp — working instructions

## Before every new user task

Run from the repository root:

```powershell
python tools/maintenance/check_upstreams.py --task "Short description of the current task"
```

This is Benjamin's explicit requirement: consult BOTH official-kryo-to/ctw-native and highcmd/chinatown-wars-psp-recomp before starting a task. It also applies to branding, documentation and tooling tasks; do not force unrelated upstream changes into them. The report is docs/upstream-status.json. Read changed files / recent commits relevant to the task, then record the conclusion in docs/UPSTREAM_REVIEW.md. Fetching alone is not a review.

- If network access or a mirror is unavailable, record that limitation; do not claim a current upstream check succeeded. Continue independent work, but defer decisions that require unavailable evidence.
- The mirrors in references/ must have their own Git roots and exact expected origins. work/upstream-ctw is a historical file snapshot, NOT a Git mirror. Never run fetch/pull there: Git can fall back to this project's parent repository.
- Do not merge/pull upstream into our runtime automatically. Compare findings, record a pinned source commit and license, adapt compatible algorithms to Rust and verify against our PSP Europe ULES-01347 1.01 evidence. Android and PSP USA are references, not interchangeable data/addresses.
- Respect upstream license/attribution. Do not send messages, open issues or submit upstream PRs unless Benjamin authorizes that communication.

## Mandatory: Keep Visual Port Status & Treemap Up To Date After Every Significant Progress

> **BENJAMIN'S CORE MANDATE FOR ALL AI MODELS (GPT, Claude, Gemini, Copilot, Cursor, etc.)**:
> After **every significant progress, new gameplay system, module expansion, or refactoring**, the visual status treemap, SVG badges, and documentation dashboards **MUST be updated and kept current**.
> Never leave the repository with stale status visuals.

### Rules for Port Progress & Treemap Maintenance:
1. **Automated Synchronization**:
   - Running `.\Compiler.ps1 -Test` automatically re-measures the codebase and synchronizes the treemap, badges, `README.md`, and `docs/index.html`.
   - You can also run the generator directly:
     ```powershell
     python tools/progress/progress.py
     ```
2. **100% Native Rust Data Only (Absolute Integrity Rule)**:
   - The visual treemap (`docs/assets/progress.svg`), the SVG badges (`docs/assets/badge-overall.svg`, `docs/assets/badge-routines.svg`), and the subsystems breakdown table in `README.md` and `docs/index.html` **MUST reflect strictly and exclusively our native Rust codebase (`src/**/*.rs`)**.
   - **NEVER** scan, borrow, or mix in third-party C++ code (e.g. from `references/ctw-native`). Every single green block in the treemap must correspond to verified, real functions implemented in pure Rust in `src/`.
3. **Updating `RUST_MODULE_MAP`**:
   - When creating a new Rust file or adding new gameplay systems in `src/`, make sure `RUST_MODULE_MAP` in `tools/progress/progress.py` associates your module with the relevant Chinatown Wars classes.
   - The generator will immediately count your new Rust functions and allocate the corresponding routines and bytes in the treemap and badges.
4. **Never Hand-Edit Generated SVGs**:
   - `docs/assets/progress.svg`, `docs/assets/badge-overall.svg`, `docs/assets/badge-routines.svg`, and `docs/assets/progress.json` are generated artifacts. Always generate them through `python tools/progress/progress.py` or `.\Compiler.ps1 -Test`.

## Handoff

Keep outputs/CTW-Rust/AI_HANDOFF_GUIDE.md current. Preserve private game data and the latest film/logos. Public branding: ReGTA CTW PC Recomp; website/README default language: English. No emulated CPU in our Rust runtime; references may use different approaches.
