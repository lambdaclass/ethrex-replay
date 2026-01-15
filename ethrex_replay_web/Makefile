.PHONY: setup run dev clean reset deps compile test format check help install-inotify

# Default target
help:
	@echo "Ethrex Replay Web - Available commands:"
	@echo ""
	@echo "  make setup    - Install dependencies and setup database (first time)"
	@echo "  make run      - Start the Phoenix server"
	@echo "  make dev      - Start server with interactive Elixir shell (iex)"
	@echo "  make deps     - Install/update dependencies"
	@echo "  make compile  - Compile the application"
	@echo "  make test     - Run tests"
	@echo "  make format   - Format code"
	@echo "  make check    - Run formatter check and compile with warnings as errors"
	@echo "  make clean    - Remove build artifacts"
	@echo "  make reset    - Reset database (drop and recreate)"
	@echo "  make assets   - Build assets (CSS/JS)"
	@echo ""

# First-time setup: install deps, create db, build assets
setup: install-inotify
	mix setup

# Install inotify-tools on Linux if not present (needed for Phoenix file watching)
install-inotify:
	@if [ "$$(uname)" = "Linux" ]; then \
		if ! command -v inotifywait >/dev/null 2>&1; then \
			echo "Installing inotify-tools..."; \
			if command -v apt-get >/dev/null 2>&1; then \
				sudo apt-get update && sudo apt-get install -y inotify-tools; \
			elif command -v dnf >/dev/null 2>&1; then \
				sudo dnf install -y inotify-tools; \
			elif command -v yum >/dev/null 2>&1; then \
				sudo yum install -y inotify-tools; \
			elif command -v pacman >/dev/null 2>&1; then \
				sudo pacman -S --noconfirm inotify-tools; \
			else \
				echo "Warning: Could not detect package manager. Please install inotify-tools manually."; \
			fi \
		else \
			echo "inotify-tools already installed"; \
		fi \
	fi

# Start the Phoenix server
run:
	mix phx.server

# Start with interactive Elixir shell
dev:
	iex -S mix phx.server

# Install/update dependencies
deps:
	mix deps.get

# Compile the application
compile:
	mix compile

# Run tests
test:
	mix test

# Format code
format:
	mix format

# Check formatting and compile with warnings as errors
check:
	mix format --check-formatted
	mix compile --warnings-as-errors

# Remove build artifacts
clean:
	rm -rf _build
	rm -rf deps
	rm -rf priv/static/assets
	rm -f *.db
	rm -f *.db-*

# Reset database (drop and recreate)
reset:
	mix ecto.reset

# Build assets
assets:
	mix assets.build

# Production build
prod-build:
	MIX_ENV=prod mix assets.deploy
	MIX_ENV=prod mix compile

# Production release
prod-release: prod-build
	MIX_ENV=prod mix release
