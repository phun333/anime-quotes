# Anime Quotes TUI

![App Screenshot](public/banner-anime-quotes.png)

## Overview

Anime Quotes TUI is a small terminal app that lets you browse iconic anime lines while showing a color ASCII rendition of each scene. Quotes and media come from `anime.toml`; the ASCII output is generated at runtime from the referenced images in `assets/`.

## Getting Started

```bash
cargo run
```

Use the arrow keys to move between quotes and press `q` to exit.

## Configuration

- Edit `anime.toml` to add or update quotes and their image paths.
- Tweak `config.toml` to change ASCII density, gradients, and UI colors.

## License

Released under the MIT License. See [LICENSE](LICENSE) for details.
