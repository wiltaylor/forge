export FORGE_PORT := "8765"

[default, private]
main:
	@just --list

# Install workspace npm dependencies
[group('build')]
frontend-install:
	pnpm install

# Build all npm packages and apps (tokens, ui, charts, graph, code, client, remote, gallery, remote-widgets)
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

# Run all test suites
[group('test')]
test: frontend-test rust-test python-test

# Run the Rust demo app (debug build reads gallery dist from disk)
[group('demo')]
rust-demo: frontend-build
	cd examples/rust-demo && cargo run -p rust-demo

# Run the Python demo app (uv single-file script)
[group('demo')]
python-demo: frontend-build
	cd examples/python-demo && uv run demo.py

# Start the gallery frontend dev server (Vite, proxies /api to FORGE_PORT)
[group('dev')]
gallery-dev:
	pnpm dev

# Start the widgets docker testenv (VNC 127.0.0.1:5900 pass "forge", RDP 127.0.0.1:3389 forge/forge)
[group('dev')]
widgets-testenv-up:
	docker compose -f examples/widgets-testenv/docker-compose.yml up -d --build

# Stop and remove the widgets docker testenv
[group('dev')]
widgets-testenv-down:
	docker compose -f examples/widgets-testenv/docker-compose.yml down
