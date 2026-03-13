<div align="center">
  <img src="data/icons/hicolor/scalable/apps/io.github.basshift.Recall.svg" width="92" alt="Recall icon" />
  <h1>Recall</h1>
  <p><strong>A memory game built with Rust, GTK4, and libadwaita that starts calm and grows into a sharp challenge.</strong></p>
  <p>Classic, Trio, and Infinite modes with polished transitions, local records, and escalating pressure at higher levels.</p>
</div>

## Preview

Classic mode gameplay:

![Recall classic gameplay](data/screenshots/recall-classic-gameplay.png)

Demo video:

- https://youtu.be/j905L9dmyVI

## Project Goals

- Create a clean GNOME-native game experience
- Keep gameplay readable and responsive across difficulty levels
- Maintain a codebase that is easy to extend and maintain

## Game Modes

- `Classic`: pair matching with progressive difficulty (`Easy`, `Medium`, `Hard`, `Expert`)
- `Trio`: group matching variant with level-based progression
- `Infinite`: continuous rounds with escalating pressure

## Core Features

- Local score/history tracking
- Animated board transitions and victory flow
- Theme support (light and dark variants)
- Debug shortcuts for rapid gameplay testing

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

## Debug Mode (Optional)

Enable debug shortcuts:

```bash
RECALL_DEBUG=1 cargo run
```

In game:

- `Ctrl+N` or `Ctrl+F9`: prepare a near-win board
- `Ctrl+R`: trigger the contextual in-game action (restart in Classic/Trio, end run in Infinite)
- `Ctrl+1/2/3/4`: force level by mode context

## Flatpak

Manifests:

- Local build: `io.github.basshift.Recall.yaml`
- Flathub submission: `io.github.basshift.Recall.flathub.yaml`

Local Flatpak build:

```bash
flatpak-builder --force-clean build-flatpak io.github.basshift.Recall.yaml
```

## Project Status

- Target release: `v1.0.0`
- Flathub packaging manifest is maintained in this repository

## Author

Sebastian Davila (Basshift)
