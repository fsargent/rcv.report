# rcv.report

A static site and data pipeline for publishing ranked-choice voting (RCV) election reports.

- Web UI: Sapper (Svelte) app in `src/` that renders published reports
- Data pipeline: Rust project in `report_pipeline/` that normalizes raw data and generates `report.json`

## Prerequisites

- Node.js 18+ (matches CI) and npm
- Rust (stable) if you need to regenerate reports

## Quick start (view existing reports)

```bash
npm install
./dev.sh
# open http://localhost:3000
```

The app reads report data from `report_pipeline/reports` via the `RANKED_VOTE_REPORTS` environment variable (set by `dev.sh`).

## Scripts

- `npm run dev`: start Sapper dev server
- `npm run build`: build the app (legacy enabled)
- `npm run export`: export a static site to `__sapper__/export`
- `./dev.sh`: run dev with `RANKED_VOTE_REPORTS="report_pipeline/reports"`
- `./build.sh`: export with `RANKED_VOTE_REPORTS` set (for local static output)

## Build and export

```bash
npm install
RANKED_VOTE_REPORTS="report_pipeline/reports" npm run build
RANKED_VOTE_REPORTS="report_pipeline/reports" npm run export
# output: __sapper__/export
```

## Deployment

Deploys are handled by GitHub Pages via `.github/workflows/deploy-rcv-report.yml`:

- On push to `main`/`master`, CI installs dependencies, builds, exports, and publishes `__sapper__/export` to Pages
- CI sets `RANKED_VOTE_REPORTS` to `${{ github.workspace }}/report_pipeline/reports`

## Regenerating reports (optional)

If you need to update or add election data, use the Rust pipeline in `report_pipeline/`.

Common tasks (see `report_pipeline/README.md` for details):

```bash
cd report_pipeline
./mount.sh   # initialize submodules / fetch data if configured
./sync.sh    # sync raw data and metadata
./report.sh  # generate normalized data and reports
```

Generated reports will appear under `report_pipeline/reports/.../report.json` and are consumed by the web UI.

## Project structure

- `src/`: Sapper app (Svelte components, routes, API endpoints)
- `static/`: static assets copied to export
- `report_pipeline/`: Rust data processing and report generation
- `__sapper__/export`: export output (gitignored)

## License

Website content and generated reports may be freely distributed with attribution under CC-BY.
