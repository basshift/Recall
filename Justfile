set shell := ["bash", "-eu", "-o", "pipefail", "-c"]

app_id := "io.github.basshift.Recall"
manifest := "io.github.basshift.Recall.yaml"
build_dir := "build-flatpak"
logs_dir := "logs"

default:
  @just help

help:
  @echo "Recall Flatpak workflow"
  @echo ""
  @echo "  just deps-flatpak         # Install GNOME 49 SDK/runtime + Rust extension"
  @echo "  just sdk-shell            # Open interactive shell inside org.gnome.Sdk//49"
  @echo "  just sdk-verify           # Run check+clippy+test inside Flatpak SDK"
  @echo "  just sdk-i18n-update      # Generate POT inside Flatpak SDK"
  @echo "  just sdk-i18n-compile     # Compile PO -> MO inside Flatpak SDK"
  @echo "  just vendor               # Refresh Cargo vendor/ for offline Flatpak build"
  @echo "  just verify               # Run cargo check + clippy (-D warnings) + test"
  @echo "  just i18n-update          # Refresh po/{{app_id}}.pot from Rust sources"
  @echo "  just i18n-compile         # Build .mo files into po/<lang>/LC_MESSAGES/"
  @echo "  just build-flatpak        # Incremental build/install (fast iteration)"
  @echo "  just build-flatpak-clean  # Clean rebuild/install (slower, deterministic)"
  @echo "  just run-flatpak          # Run app exactly as packaged"
  @echo "  just run-flatpak-debug    # Run with GTK/Glib debug + Rust backtrace"
  @echo "  just logs-flatpak         # Show tail of latest Flatpak build log"
  @echo "  just logs-flatpak-follow  # Follow latest Flatpak build log"

deps-flatpak:
  flatpak --user install -y flathub \
    org.gnome.Platform//49 \
    org.gnome.Sdk//49 \
    org.freedesktop.Sdk.Extension.rust-stable//25.08

sdk-shell:
  flatpak run --user --devel --command=sh \
    --filesystem=host \
    --env=PATH=/usr/lib/sdk/rust-stable/bin:/usr/bin \
    org.gnome.Sdk//49 \
    -i

sdk-verify:
  flatpak run --user --devel --command=bash \
    --filesystem=host \
    --env=PATH=/usr/lib/sdk/rust-stable/bin:/usr/bin \
    org.gnome.Sdk//49 \
    -lc "cd \"$PWD\" && cargo check && cargo clippy -- -D warnings && cargo test"

sdk-i18n-update:
  flatpak run --user --devel --command=bash \
    --filesystem=host \
    --env=PATH=/usr/lib/sdk/rust-stable/bin:/usr/bin \
    org.gnome.Sdk//49 \
    -lc "cd \"$PWD\" && python3 scripts/i18n_update.py"

sdk-i18n-compile:
  flatpak run --user --devel --command=bash \
    --filesystem=host \
    --env=PATH=/usr/lib/sdk/rust-stable/bin:/usr/bin \
    org.gnome.Sdk//49 \
    -lc "cd \"$PWD\" && for lang in $(cat po/LINGUAS); do mkdir -p \"po/$lang/LC_MESSAGES\"; msgfmt \"po/$lang.po\" -o \"po/$lang/LC_MESSAGES/{{app_id}}.mo\"; done"

vendor:
  cargo vendor --locked vendor > /tmp/recall-vendor-config.toml
  test -d vendor

verify:
  cargo check
  cargo clippy -- -D warnings
  cargo test

i18n-update:
  command -v python3 >/dev/null || { echo "python3 not found."; exit 1; }
  python3 scripts/i18n_update.py

i18n-compile:
  command -v msgfmt >/dev/null || { echo "msgfmt not found. Install gettext package."; exit 1; }
  @for lang in $(cat po/LINGUAS); do \
    mkdir -p "po/${lang}/LC_MESSAGES"; \
    msgfmt "po/${lang}.po" -o "po/${lang}/LC_MESSAGES/{{app_id}}.mo"; \
  done

build-flatpak:
  mkdir -p {{logs_dir}}; \
  stamp="$$(date +%F-%H%M)"; \
  stdbuf -oL -eL flatpak-builder \
    --user --install --ccache --force-clean \
    --install-deps-from=flathub \
    {{build_dir}} {{manifest}} \
    2>&1 | tee "{{logs_dir}}/flatpak-build-$${stamp}.log"

build-flatpak-clean:
  mkdir -p {{logs_dir}}; \
  stamp="$$(date +%F-%H%M)"; \
  stdbuf -oL -eL flatpak-builder \
    --user --install --ccache --force-clean --delete-build-dirs \
    --install-deps-from=flathub \
    {{build_dir}} {{manifest}} \
    2>&1 | tee "{{logs_dir}}/flatpak-build-$${stamp}.log"

run-flatpak:
  flatpak run {{app_id}}

run-flatpak-debug:
  RUST_BACKTRACE=1 G_MESSAGES_DEBUG=all flatpak run {{app_id}}

logs-flatpak:
  log="$$(ls -1t {{logs_dir}}/flatpak-build-*.log 2>/dev/null | head -n1 || true)"; \
  test -n "$${log}" || { echo "No build logs found in {{logs_dir}}/"; exit 1; }; \
  tail -n 120 "$${log}"

logs-flatpak-follow:
  log="$$(ls -1t {{logs_dir}}/flatpak-build-*.log 2>/dev/null | head -n1 || true)"; \
  test -n "$${log}" || { echo "No build logs found in {{logs_dir}}/"; exit 1; }; \
  tail -f "$${log}"
