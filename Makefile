BINARY := mate

.PHONY: all build install setup clean test download

all: build

build:
	cargo build --release

install:
	cargo install --path .

download:
	@echo "Pulling Ollama models..."
	ollama pull llama3.2
	ollama pull qwen2.5:0.5b
	@echo ""
	@echo "Models ready!"

setup: download install
	@mate init
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
