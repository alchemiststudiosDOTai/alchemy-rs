.PHONY: help fmt check clippy test complexity duplicates large-files dead-code quality-full quality-quick

help: ## Show this help message
	@echo "Alchemy - Development Commands"
	@echo ""
	@echo "Quality Checks:"
	@echo "  make quality-quick    - Fast checks (fmt, clippy, check)"
	@echo "  make quality-full     - All checks including complexity, duplicates, dead-code"
	@echo "  make complexity       - Run cyclomatic complexity analysis"
	@echo "  make duplicates       - Run duplicate code detection"
	@echo "  make dead-code        - Run dead code detection"
	@echo "  make large-files      - Detect large files (default: 500KB)"
	@echo ""
	@echo "Standard Commands:"
	@echo "  make fmt              - Format code"
	@echo "  make check            - Check compilation"
	@echo "  make clippy           - Run clippy linter"
	@echo "  make test             - Run tests"

fmt: ## Format code with rustfmt
	cargo fmt --all

check: ## Check compilation
	cargo check --all-targets --all-features

clippy: ## Run clippy with complexity checks
	cargo clippy --all-targets --all-features -- -D warnings \
		-W clippy::cognitive_complexity \
		-W clippy::too_many_lines \
		-W clippy::too_many_arguments

test: ## Run all tests
	cargo test --all-features

complexity: ## Run cyclomatic complexity analysis
	@echo "Checking cognitive complexity with clippy..."
	@cargo clippy --all-targets --all-features -- \
		-W clippy::cognitive_complexity \
		-W clippy::too_many_lines \
		2>&1 | grep -E "(cognitive_complexity|too_many_lines)" || echo "✓ No complexity violations found"

duplicates: ## Run duplicate code detection
	@echo "Running duplicate code detection with polydup..."
	@command -v polydup >/dev/null 2>&1 || { echo "polydup not installed. Run: cargo install polydup-cli"; exit 1; }
	@polydup --min-tokens 100 --similarity 0.9 src/ || echo "✓ No significant duplicates found"

dead-code: ## Run dead code detection
	@echo "Checking for dead code..."
	@cargo clippy --all-targets --all-features -- -W dead_code -W unused 2>&1 | grep -E "(dead_code|is never used|unused)" || echo "✓ No dead code detected"

large-files: ## Detect large files (default: 500KB)
	@echo "Checking for large files (>500KB)..."
	@find src -type f -size +500k -exec ls -lh {} \; 2>/dev/null | awk '{ print $$9 ": " $$5 }' || echo "✓ No large files found"

quality-quick: fmt clippy check ## Fast quality checks (pre-commit)
	@echo "✓ Quick quality checks passed"

quality-full: quality-quick test complexity duplicates dead-code large-files ## All quality checks
	@echo "✓ Full quality analysis complete"
