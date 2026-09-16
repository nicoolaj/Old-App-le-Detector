# SPDX-License-Identifier: MIT

.DEFAULT_GOAL := help
TARGET := universal-apple-darwin

help:  ## Affiche cette aide
	@grep -E '^[a-zA-Z_-]+:.*## ' $(MAKEFILE_LIST) | awk 'BEGIN{FS=":.*## "}{printf "  %-6s %s\n", $$1, $$2}'

secu:  ## Sécurité + bonnes pratiques (fmt, clippy, cargo audit)
	cargo fmt --manifest-path src-tauri/Cargo.toml --check
	cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
	@cargo audit --version >/dev/null 2>&1 || cargo install cargo-audit --locked
	cd src-tauri && cargo audit

app: _tauri-cli  ## Construit le .app universel (arm64 + x86_64)
	cargo tauri build --target $(TARGET) --bundles app

pkg: _tauri-cli  ## Construit le .dmg (inclut le .app)
	cargo tauri build --target $(TARGET) --bundles dmg

clean:  ## Supprime les artefacts de build (src-tauri/target)
	cd src-tauri && cargo clean

mrproper: clean  ## Nettoyage complet (+ fichiers générés par Tauri : src-tauri/gen)
	rm -rf src-tauri/gen

_tauri-cli:
	@cargo tauri --version >/dev/null 2>&1 || cargo install tauri-cli --locked

.PHONY: help secu app pkg clean mrproper _tauri-cli
