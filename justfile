export FORGE_PORT := "8765"

[default, private]
main:
	@just --list

# Install workspace npm dependencies
[group('build')]
frontend-install:
	pnpm install

# Build all npm packages and apps (tokens, ui, charts, graph, code, client, remote, gallery, remote-widgets, auth)
[group('build')]
frontend-build: frontend-install
	pnpm build

# Build the Rust workspace (debug)
[group('build')]
rust-build:
	cargo build

# Build the rust-demo release binary with the frontend embedded (single binary)
[group('build')]
rust-demo-build: frontend-build
	cargo build --release -p rust-demo

# Build the forge-tauri plugin crate with all widgets (own workspace — heavy tauri tree)
[group('build')]
tauri-build:
	cargo build --manifest-path crates/forge-tauri/Cargo.toml --features widgets

# Build tauri-demo release bundles (deb + AppImage; NO_STRIP: linuxdeploy chokes on new binutils)
[group('build')]
tauri-demo-build: frontend-install
	cd examples/tauri-demo && NO_STRIP=true pnpm tauri build

# Build the forge-auth release binary (embeds apps/auth/dist)
[group('build')]
auth-build: frontend-build
	cargo build --release -p forge-auth

# Build everything
[group('build')]
build: frontend-build rust-build

# Run npm package tests
[group('test')]
frontend-test: frontend-install
	pnpm test

# Run Rust tests
[group('test')]
rust-test:
	cargo test

# Run Python package tests
[group('test')]
python-test:
	uv run --project python/forge-server --extra dev pytest python/forge-server/tests

# Run the black-box contract parity suite against a live server (FORGE_TEST_BASE_URL)
[group('test')]
parity-test base_url='http://127.0.0.1:8765':
	FORGE_TEST_BASE_URL={{base_url}} uv run --with 'httpx>=0.27' --with 'pytest>=8' --with 'websockets>=13' pytest examples/parity

# Run forge-tauri tests (own workspace — heavy tauri tree, so not part of `just test`)
[group('test')]
tauri-test:
	cargo test --manifest-path crates/forge-tauri/Cargo.toml --features widgets

# Run the forge-auth dev-login gating tests with the feature compiled in
[group('test')]
auth-test-devlogin:
	cargo test -p forge-auth --features dev-login

# Full end-to-end OIDC flow test against a real server (builds the debug binary itself)
[group('test')]
auth-e2e-test:
	uv run scripts/oidc_flow_test.py

# Run all test suites
[group('test')]
test: frontend-test rust-test python-test auth-test-devlogin auth-e2e-test

# Run the Rust demo app (debug build reads gallery dist from disk)
[group('demo')]
rust-demo: frontend-build
	cd examples/rust-demo && cargo run -p rust-demo

# Run the Python demo app (uv single-file script)
[group('demo')]
python-demo: frontend-build
	cd examples/python-demo && uv run demo.py

# Run the Tauri demo app (native window; contract + widgets over pure IPC)
[group('demo')]
tauri-demo: frontend-install
	cd examples/tauri-demo && pnpm tauri dev

# Start the gallery frontend dev server (Vite, proxies /api to FORGE_PORT)
[group('dev')]
gallery-dev:
	pnpm dev

# Run the forge-auth IdP in debug with dev-login compiled in (serves apps/auth/dist from disk, :8770)
[group('dev')]
auth-dev:
	FORGE_PORT=8770 cargo run -p forge-auth --features dev-login

# Run the forge-auth IdP in debug WITHOUT dev-login (exercise real login flows)
[group('dev')]
auth-dev-prod:
	FORGE_PORT=8770 cargo run -p forge-auth

# Start the auth frontend dev server on :5174 (proxies /api, /oauth2, /.well-known to :8770)
[group('dev')]
auth-frontend-dev:
	FORGE_PORT=8770 pnpm --filter @forge/auth dev

# Delete the local forge-auth dev SQLite database (migrations re-run on next start)
[group('dev')]
auth-db-reset:
	rm -f data/forge-auth.db data/forge-auth.db-journal data/forge-auth.db-wal data/forge-auth.db-shm

# Build the forge-auth production container image
[group('docker')]
auth-docker-build tag='latest':
	docker build -f crates/forge-auth/Dockerfile -t forge-auth:{{tag}} .

# Build the forge-auth DEV container image with password-less dev-login compiled in
[group('docker')]
auth-docker-build-dev:
	docker build -f crates/forge-auth/Dockerfile --build-arg FEATURES=dev-login -t forge-auth:dev .

# Start the forge-auth compose stack (SQLite volume; Postgres variant inside)
[group('docker')]
auth-docker-up:
	docker compose -f crates/forge-auth/docker-compose.yml up -d

# Stop the forge-auth compose stack
[group('docker')]
auth-docker-down:
	docker compose -f crates/forge-auth/docker-compose.yml down

# Start the widgets docker testenv (VNC 127.0.0.1:5900 pass "forge", RDP 127.0.0.1:3389 forge/forge)
[group('dev')]
widgets-testenv-up:
	docker compose -f examples/widgets-testenv/docker-compose.yml up -d --build

# Stop and remove the widgets docker testenv
[group('dev')]
widgets-testenv-down:
	docker compose -f examples/widgets-testenv/docker-compose.yml down
