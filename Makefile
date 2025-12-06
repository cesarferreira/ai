BINARY := aisuggest
PREFIX ?= /usr/local/bin
BUILD_DIR := target/release
ARTIFACT := $(BUILD_DIR)/$(BINARY)

.PHONY: all build install clean snippet ask test

all: build

build:
	cargo build --release

install: build
	cp "$(ARTIFACT)" "$(PREFIX)/$(BINARY)"

snippet:
	cat zsh_integration_snippet.txt

ask: build
	@intent="$${Q}"; \
	if [ -z "$${intent}" ]; then \
		printf "Intent: "; \
		IFS= read -r intent; \
	fi; \
	if [ -z "$${intent}" ]; then \
		echo "No intent provided."; exit 1; \
	fi; \
	$(ARTIFACT) "$${intent}"

clean:
	cargo clean

test: build
	@echo "==> Asking: 'how many lines in README.md'"
	@echo "Command: $$($(ARTIFACT) "how many lines in README.md")"
	@echo ""
	@echo "==> Actual answer:"
	@wc -l README.md
