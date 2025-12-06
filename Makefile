BINARY := ai

.PHONY: all build install setup clean test

all: build

build:
	cargo build --release

install:
	cargo install --path .

setup: install
	@ai init
	@echo ""
	@echo "Run: source ~/.zshrc  (or restart your shell)"

clean:
	cargo clean

test: build
	@echo "==> Asking: 'how many lines in README.md'"
	@echo "Command: $$(./target/release/$(BINARY) "how many lines in README.md")"
	@echo ""
	@echo "==> Actual answer:"
	@wc -l README.md
