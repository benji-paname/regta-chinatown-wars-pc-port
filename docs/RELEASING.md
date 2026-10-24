# Release process

Current development release: **0.1.2-alpha**. This restructuring creates local commits; publishing remains a separate action.

1. Update the package version in `outputs/CTW-Rust/Cargo.toml`, the root lockfile, compiler report version, README and CHANGELOG together.
2. Keep unfinished entries under `Unreleased`. Move completed changes to a dated release entry without inventing past releases.
3. Run `Compiler.ps1 -Test`: no failures, ignored/filtered tests or emitted compiler warnings. Record toolchain, executable SHA-256 and limits of validation.
4. Run a clean source-only checkout test without ISO/tables. Run private integration and demo smoke checks separately when local inputs are available.
5. Review tracked files with `tools/maintenance/check_repository.py`. Do not include game data, captures, local tables, ISO/ELF files or build caches in source/release archives.
6. Update `outputs/CTW-Rust/AI_HANDOFF_GUIDE.md` with a timestamp, exact results and next command. Commit the verified state. Create an annotated version tag only for an intentional release.
7. Before any public binary distribution, include the corresponding GPL source/license/notices and inspect the archive. A local successful build is not evidence that a public release was uploaded.

Alpha patch releases may fix individual systems or infrastructure. Move to beta only when a documented gameplay scope is complete and reproducibly tested. Use release candidates for regression stabilization. Do not announce a 1.0 release based only on unit-test counts.

Support links: [Tipeee](https://fr.tipeee.com/benji-paname/) and [PayPal.Me](https://paypal.me/BenjiLsg), supplied by the owner. They fund development; they must not be presented as a game purchase or a source of proprietary assets.
