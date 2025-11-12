# Makefile for cmd2ai

# Default installation prefix
PREFIX ?= $(HOME)/.local

# Installation directories
BINDIR = $(PREFIX)/bin
ZSHDIR = $(HOME)/.config/zsh/functions

# Optional components
WITH_WEBSEARCH ?= 0

# Binary name
BINARY = ai

# Default target
.PHONY: all
all: build

# Build the release binary
.PHONY: build
build:
	cargo build --release

# Install the binary and ZSH widget
.PHONY: install
install: build
	@echo "Installing ai binary to $(BINDIR)..."
	@mkdir -p $(BINDIR)
	@cp target/release/$(BINARY) $(BINDIR)/
	@chmod +x $(BINDIR)/$(BINARY)
	@echo "Installing ZSH widget to $(ZSHDIR)..."
	@mkdir -p $(ZSHDIR)
	@cp ai-widget.zsh $(ZSHDIR)/
	@if [ "$(WITH_WEBSEARCH)" = "1" ] || [ "$(WITH_WEBSEARCH)" = "yes" ]; then \
		echo "Installing websearch script (ask) to $(BINDIR)..."; \
		cp scripts/ask $(BINDIR)/ask; \
		chmod +x $(BINDIR)/ask; \
	fi
	@echo ""
	@echo "Installation complete!"
	@echo ""
	@echo "To use the ai command, ensure $(BINDIR) is in your PATH:"
	@echo "  export PATH=\"$(BINDIR):\$$PATH\""
	@echo ""
	@echo "To use the ZSH widget, add this to your ~/.zshrc:"
	@echo "  source $(ZSHDIR)/ai-widget.zsh"
	@echo ""

# Install including websearch function
.PHONY: install-websearch
install-websearch: WITH_WEBSEARCH=1
install-websearch: install

# Uninstall
.PHONY: uninstall
uninstall:
	@echo "Removing ai binary..."
	@rm -f $(BINDIR)/$(BINARY)
	@echo "Removing ZSH widget..."
	@rm -f $(ZSHDIR)/ai-widget.zsh
	@echo "Removing websearch script (if present)..."
	@rm -f $(BINDIR)/ask
	@echo "Uninstall complete!"

# Clean build artifacts
.PHONY: clean
clean:
	cargo clean

# Development build
.PHONY: dev
dev:
	cargo build

# Run tests
.PHONY: test
test:
	cargo test

# Format code
.PHONY: fmt
fmt:
	cargo fmt

# Run clippy
.PHONY: lint
lint:
	cargo clippy

# Check code
.PHONY: check
check:
	cargo check

# Help target
.PHONY: help
help:
	@echo "Available targets:"
	@echo "  make          - Build release binary (default)"
	@echo "  make install  - Build and install binary and ZSH widget"
	@echo "                  Add WITH_WEBSEARCH=1 to also install 'ask' script to $(BINDIR)"
	@echo "  make install-websearch - Install with websearch script enabled (same as WITH_WEBSEARCH=1)"
	@echo "  make uninstall- Remove installed files"
	@echo "  make clean    - Clean build artifacts"
	@echo "  make dev      - Build debug binary"
	@echo "  make test     - Run tests"
	@echo "  make fmt      - Format code"
	@echo "  make lint     - Run clippy linter"
	@echo "  make check    - Check compilation"
	@echo ""
	@echo "Installation prefix can be changed with PREFIX:"
	@echo "  make install PREFIX=/usr/local"