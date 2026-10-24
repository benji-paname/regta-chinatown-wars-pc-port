# Publish ReGTA CTW

The intended repository name is **regta-ctw**. No GitHub account name or remote URL has been assumed. The website is static and lives in `docs/`; it needs no build service or dependency installation.

## Two-line setup

1. At https://github.com/new, create a **Public** repository named **regta-ctw**, leaving README, license and .gitignore initialization unchecked.
2. From the local project root, run `git remote add origin https://github.com/YOUR_USERNAME/regta-ctw.git` then `git push -u origin main`; in **Settings → Pages**, choose **Deploy from a branch → main → /docs → Save**.

Replace `YOUR_USERNAME` with your actual GitHub username. GitHub may ask you to sign in. If `origin` already exists, inspect it with `git remote -v` before changing it; do not add a duplicate or overwrite an unrelated remote.

The expected site address is `https://YOUR_USERNAME.github.io/regta-ctw/`. This is a template, not a live deployment. On that standard Pages address, the site's GitHub links automatically resolve to the actual account and repository. A custom domain will need explicit repository links.

## Before the first public push

Review `git ls-files` to ensure that only intended source, documentation and permitted assets are tracked. Private game files, decrypted executables, research data and fonts without redistribution permission must stay outside the repository. An ignore rule does not remove files already tracked in Git history.

Finalize the font provenance described in [BRANDING.md](BRANDING.md) before presenting the logo as matching the original cover. Review the page on desktop and mobile; no browser visual verification has been completed in the current editing environment.

No public playable release is being published by this workflow. The website is a presentation of work in progress. Keep donations clearly separate from game purchases or access promises.
