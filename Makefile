BINARY := aisuggest
PREFIX ?= /usr/local/bin
BUILD_DIR := .build/release
ARTIFACT := $(BUILD_DIR)/$(BINARY)

.PHONY: all build install clean snippet ask

all: build

build:
	swift build -c release

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
	swift package clean
