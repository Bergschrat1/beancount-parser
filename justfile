set dotenv-load

@_choose:
	just --list --unsorted

# Perform all verifications (compile, test, lint, etc.)
verify: test lint doc check-msrv
	cargo deny check licenses

# Verify that everything is ready for realease (incl. secrets required for the release process)
verify-for-release: verify check-msrv
	cargo publish --dry-run
	test $GITHUB_TOKEN
	test $CARGO_REGISTRY_TOKEN

# Watch the source files and run `just verify` when source changes
watch:
	cargo watch --delay 0.1 --clear --why -- just verify

# Run the tests
test:
	cargo hack test --feature-powerset

# Run the static code analysis
lint:
	cargo fmt -- --check
	cargo hack clippy --all-targets

# Build the documentation
doc *args:
	cargo doc --all-features --no-deps {{args}}

# Open the documentation page
doc-open: (doc "--open")

# Make sure the MSRV is satisfiable
check-msrv:
	cargo msrv verify

# Clean up compilation output
clean:
	rm -rf target
	rm -f Cargo.lock
	rm -rf node_modules

# Install cargo dev-tools used by the `verify` recipe (requires rustup to be already installed)
install-dev-tools:
	rustup install stable
	rustup override set stable
	cargo install cargo-hack cargo-watch cargo-msrv

# Install a git hook to run tests before every commits
install-git-hooks:
	echo '#!/usr/bin/env sh' > .git/hooks/pre-commit
	echo 'just verify' >> .git/hooks/pre-commit
	chmod +x .git/hooks/pre-commit

generate-changelog tag:
	git cliff --unreleased --tag {{tag}} --strip header

# run the release process in dry run mode (requires npm and a `GITHUB_TOKEN`)
release-dry-run: (release "--dry-run")

# Run the release process (requires `npm`, a `GITHUB_TOKEN` and a `CARGO_REGISTRY_TOKEN`)
release *args:
	npm install --no-save @release-it/keep-a-changelog@3 @release-it/bumper@4 @j-ulrich/release-it-regex-bumper@4
	release-it {{args}}

publish:
    cargo publish
