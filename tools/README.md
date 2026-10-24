# Tools

- `build/Compile.ps1`: canonical compiler/test/delivery implementation; use root `Compiler.ps1`.
- `maintenance/audit_storage.py`: immutable-file snapshot, exact decoded-pixel BMP/PNG comparison, preservation verification. Reports stay in `work/restructure/`.
- `maintenance/Clean-VerifiedStorage.ps1`: validates the cleanup plan and removes only the authorized regenerable caches and verified redundant BMPs.
- `maintenance/check_repository.py`: checks every tracked file against the public source policy.
- `research/README.md`: index of preserved research scripts. Their originals remain intact in `work/` because many depend on their original path and private inputs.

No tool downloads proprietary game files. The image-comparison audit requires Pillow; the build and source-policy check do not.
