# RecallBench — Future Phases

## Phase: Cloudflare Pages Dashboard

Deploy the web UI as a shared community dashboard where users upload benchmark results for comparison.

### Architecture
- **Cloudflare Pages** (free) — hosts the Solid.js + Core UI web app
- **D1 database** (free tier: 5GB, 5M reads/day) — stores uploaded results and user metadata
- **No server-side compute** — users run benchmarks locally with their own LLM, upload JSONL results
- **Estimated cost: $0/month** on free tier

### What to Build
1. Add D1-backed API endpoints to the existing axum web server (or rewrite as a Cloudflare Worker for the API layer)
2. Result upload endpoint: `POST /api/results` accepts JSONL, validates, stores in D1
3. Public leaderboard view: aggregates scores across uploaded results
4. User identification: GitHub OAuth or anonymous with a display name
5. `recallbench upload` CLI subcommand: uploads local JSONL to the hosted dashboard
6. Deploy Solid.js UI to Pages via `wrangler pages deploy`

### Pages Deployment Steps
```bash
# Build the UI
cd recallbench/src/web/ui
npm run build

# Deploy to Cloudflare Pages
npx wrangler pages deploy dist --project-name recallbench

# Set up custom domain (optional)
# recallbench.dev or bench.mindcore.dev
```

### D1 Schema
```sql
CREATE TABLE results (
    id TEXT PRIMARY KEY,
    system_name TEXT NOT NULL,
    dataset TEXT NOT NULL,
    variant TEXT NOT NULL,
    task_averaged REAL,
    overall REAL,
    total_questions INTEGER,
    total_correct INTEGER,
    per_type_json TEXT,
    uploaded_by TEXT,
    uploaded_at TEXT DEFAULT (datetime('now')),
    raw_jsonl TEXT
);

CREATE TABLE leaderboard (
    system_name TEXT,
    dataset TEXT,
    best_task_averaged REAL,
    best_overall REAL,
    run_count INTEGER,
    last_updated TEXT
);
```

---

## Phase: npm Installer

Publish recallbench as an npm package so users without Rust can install it.

### Approach: Binary Wrapper (Industry Standard)

Same pattern as esbuild, turbo, biome, tailwindcss-cli. The npm package contains no Rust — just a postinstall script that downloads the prebuilt binary for the user's platform.

```bash
npm install -g recallbench
recallbench run --system echo --dataset longmemeval
```

### What to Build

1. **GitHub Actions release workflow** — on git tag, build release binaries:
   - `recallbench-darwin-arm64` (macOS Apple Silicon)
   - `recallbench-darwin-x64` (macOS Intel)
   - `recallbench-linux-x64` (Linux x64)
   - `recallbench-linux-arm64` (Linux ARM)
   - `recallbench-win-x64.exe` (Windows)
   - Upload all to GitHub Releases

2. **npm package** (`packages/recallbench-npm/`):
   ```json
   {
     "name": "recallbench",
     "version": "0.4.0",
     "bin": { "recallbench": "bin/recallbench" },
     "scripts": {
       "postinstall": "node install.js"
     }
   }
   ```

3. **install.js** — postinstall script:
   - Detect platform (`process.platform`, `process.arch`)
   - Download matching binary from GitHub Releases
   - Place in `bin/` directory
   - Make executable (`chmod +x`)

4. **Platform packages** (optional, for faster install):
   - `@recallbench/darwin-arm64`
   - `@recallbench/darwin-x64`
   - `@recallbench/linux-x64`
   - etc.
   - Main `recallbench` package has optional deps on these

### Release Workflow
```yaml
# .github/workflows/release.yml
name: Release
on:
  push:
    tags: ['v*']

jobs:
  build:
    strategy:
      matrix:
        include:
          - target: aarch64-apple-darwin
            os: macos-latest
            name: recallbench-darwin-arm64
          - target: x86_64-apple-darwin
            os: macos-latest
            name: recallbench-darwin-x64
          - target: x86_64-unknown-linux-gnu
            os: ubuntu-latest
            name: recallbench-linux-x64
          - target: x86_64-pc-windows-msvc
            os: windows-latest
            name: recallbench-win-x64.exe
    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          targets: ${{ matrix.target }}
      - run: cargo build --release --target ${{ matrix.target }} -p recallbench
      - uses: softprops/action-gh-release@v2
        with:
          files: target/${{ matrix.target }}/release/recallbench*

  publish-npm:
    needs: build
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
        with:
          registry-url: 'https://registry.npmjs.org'
      - run: cd packages/recallbench-npm && npm publish
        env:
          NODE_AUTH_TOKEN: ${{ secrets.NPM_TOKEN }}
```

### Cost
- GitHub Releases hosting: free
- npm publishing: free
- GitHub Actions CI: free for public repos (2,000 minutes/month for private)
- **Total: $0/month**

---

## Why Not WASM (Option 1 / Approach B)

Investigated and rejected:
- MindCore uses rusqlite (SQLite FFI) which doesn't compile to WASM cleanly
- CLI subprocess spawning (`claude --print`) doesn't work in WASM/browser
- Cloudflare Workers have 30-second CPU limits — a 500-question benchmark takes 30-60 minutes
- The engineering effort to make it work exceeds the benefit vs. the binary wrapper approach

## Why Not Full SaaS (Option 3)

Investigated and rejected for v1:
- Server-side benchmark compute requires a VPS ($5-15/month)
- LLM API costs scale with users ($2-5 per full run × N users)
- Requires auth, billing, abuse prevention, job queuing
- May be worth building later if community adoption is high
- For now, users run locally (free) and upload results (free)
