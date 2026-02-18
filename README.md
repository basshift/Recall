# Recall

Recall is a memory game built with Rust, GTK4, and libadwaita.

## Features

- Classic mode (`Easy`, `Normal`, `Hard`, `Expert`)
- Tri mode (group matching)
- Infinite mode
- Local score and history tracking
- Animated board transitions and victory flow

## Tech Stack

- Rust
- GTK4
- libadwaita
- Cairo / Pango

## Run Locally

```bash
cargo run
```

Release build:

```bash
cargo build --release
```

## Debug Shortcuts (optional)

Enable debug mode:

```bash
RECALL_DEBUG=1 cargo run
```

In game:

- `Ctrl+N` or `Ctrl+F9`: prepare near-win board
- `Ctrl+R`: next round (Infinite) or quick restart (other modes)
- `Ctrl+1/2/3/4`: force level by mode context

## Flatpak

Manifest:

- `io.basshift.Recall.yaml`

Local build example:

```bash
flatpak-builder --force-clean build-flatpak io.basshift.Recall.yaml
```
