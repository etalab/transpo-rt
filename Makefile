build: ## Install or update dependencies and builds the project
	cargo build

test: ## Run the tests
	cargo test

lint: ## Checks the formatting
	cargo fmt --all -- --check

clippy: ## Checks for code style improvements
	cargo clippy -- -D warnings

clean: ## Remove the target directory
	cargo clean

check: test lint clippy ## Runs all the tests and checks

help:
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-30s\033[0m %s\n", $$1, $$2}'

.PHONY: build test lint clippy clean check help
