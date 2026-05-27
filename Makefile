.PHONY: publish build test lint format

publish:
	bash scripts/publish.sh

build:
	cargo build --release

test:
	cargo test --workspace --all-features

lint:
	cargo clippy --release -- -D warnings

format:
	cargo fmt --all -- --check

check:
	cargo check --release

clean:
	cargo clean

proto:
	@proto_files="$$(find proto -type f -name '*.proto' 2>/dev/null)"; \
	if [ -z "$$proto_files" ]; then \
		echo "No .proto files found under proto/"; \
	else \
		bash scripts/protoc-wrapper.sh --rust_out=src/ $$proto_files; \
	fi