# ReGTA CTW PC Recomp — AI Assistant Guidelines

## Core Project Mandates

1. **Keep Visual Port Status & Treemap Up To Date After Every Significant Advance**:
   - Whenever you implement a new feature, add gameplay systems, or complete refactoring, **you MUST update the port progress visual treemap, SVG badges, and documentation dashboards**.
   - Run:
     ```powershell
     python tools/progress/progress.py
     ```
     Or simply run:
     ```powershell
     .\Compiler.ps1 -Test
     ```
     (which compiles with warnings as errors, runs the 160 unit tests, and automatically updates the treemap and badges).

2. **100% Native Rust Data Only (No Borrowed or False Numbers)**:
   - The status treemap (`docs/assets/progress.svg`), the SVG badges (`badge-overall.svg`, `badge-routines.svg`), and the breakdown tables in `README.md` and `docs/index.html` **must reflect strictly and exclusively our native Rust codebase (`src/**/*.rs`)**.
   - **NEVER** scan, borrow, or mix in third-party C++ code (e.g. from `references/ctw-native`). Every green block must correspond to verified, real functions implemented in pure Rust in `src/`.

3. **Updating Module Mapping**:
   - If you create a new Rust file in `src/`, add it to `RUST_MODULE_MAP` in `tools/progress/progress.py` so that its functions are properly allocated to the corresponding Chinatown Wars classes.

4. **Zero Warnings & Test Pass Guarantee**:
   - The Rust build is configured with `-D warnings`. Any warning causes compilation failure.
   - All 160 unit tests must always pass before any commit.
