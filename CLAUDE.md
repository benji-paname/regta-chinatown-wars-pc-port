# ReGTA CTW PC Recomp — Instructions for AI Models

## Benjamin's Core Mandate: Keep Visual Port Status & Treemap Up To Date

After **every significant progress, new gameplay system, module expansion, or milestone**:
1. **Regenerate the visual status treemap, badges and dashboards**:
   ```powershell
   python tools/progress/progress.py
   # Or automatically:
   .\Compiler.ps1 -Test
   ```
2. **100% Native Rust Data Only**:
   - The visual treemap (`docs/assets/progress.svg`), the SVG badges (`docs/assets/badge-overall.svg`, `docs/assets/badge-routines.svg`), and the subsystems breakdown table in `README.md` and `docs/index.html` **MUST reflect strictly and exclusively our native Rust codebase (`src/**/*.rs`)**.
   - **NEVER** scan, borrow, or mix in third-party C++ code (e.g. from `references/ctw-native`). Every green block must correspond to verified, real functions implemented in pure Rust in `src/`.
3. **Keep `RUST_MODULE_MAP` Updated**:
   - When creating a new Rust file or adding new gameplay systems in `src/`, make sure `RUST_MODULE_MAP` in `tools/progress/progress.py` associates your module with the relevant Chinatown Wars classes.
4. **Never Leave Stale Status Visuals**:
   - A task with new gameplay, systems or tests is NOT complete until `Compiler.ps1 -Test` or `python tools/progress/progress.py` has run.

## Build & Test Commands

```powershell
# Compile and test with warnings treated as errors, and synchronize visual status:
.\Compiler.ps1 -Test

# Only synchronize progress visual treemap and badges:
python tools/progress/progress.py
```
