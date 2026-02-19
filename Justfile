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
  @echo "  just vendor               # Refresh Cargo vendor/ for offline Flatpak build"
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

vendor:
  cargo vendor --locked vendor > /tmp/recall-vendor-config.toml
  test -d vendor

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
