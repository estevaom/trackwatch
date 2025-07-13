.PHONY: all build test clean run release format lint coverage install help

# Default target
all: format lint test build

# Build the project
build:
	@echo "Building project..."
	@cargo build

# Build release version
release:
	@echo "Building release version..."
	@cargo build --release

# Run tests
test:
	@echo "Running tests..."
	@cargo test

# Run the application
run:
	@echo "Running trackwatch..."
	@cargo run --release

# Format code
format:
	@echo "Formatting code..."
	@cargo fmt

# Run linter
lint:
	@echo "Running clippy..."
	@cargo clippy -- -D warnings

# Generate coverage report
coverage:
	@echo "Generating coverage report..."
	@cargo tarpaulin --out Html
	@echo "Coverage report generated: tarpaulin-report.html"

# Clean build artifacts
clean:
	@echo "Cleaning build artifacts..."
	@cargo clean
	@rm -f cobertura.xml tarpaulin-report.html lcov.info

# Install the binary locally
install:
	@echo "Installing trackwatch..."
	@cargo install --path .

# Check if all dependencies are available
check-deps:
	@echo "Checking dependencies..."
	@command -v playerctl >/dev/null 2>&1 || { echo "playerctl is required but not installed."; exit 1; }
	@echo "All dependencies are installed!"

# Help
help:
	@echo "Available targets:"
	@echo "  make build     - Build the project"
	@echo "  make release   - Build release version"
	@echo "  make test      - Run tests"
	@echo "  make run       - Run the application"
	@echo "  make format    - Format code"
	@echo "  make lint      - Run clippy linter"
	@echo "  make coverage  - Generate coverage report"
	@echo "  make clean     - Clean build artifacts"
	@echo "  make install   - Install the binary locally"
	@echo "  make check-deps- Check if dependencies are installed"
	@echo "  make help      - Show this help message"
