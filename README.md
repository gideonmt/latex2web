# latex2web

Turns LaTeX documents into good looking HTML pages.
Work in progress.

## Dependencies

Install LatexML:

```bash
# mac
brew install latexml

# ubuntu/debian
sudo apt install latexml

# arch
sudo pacman -S perl-latexml
```

## Build

```bash
cargo build --release
```

## usage

```bash
# basic
latex2web document.tex

# specify output
latex2web document.tex -o out.html

# use dark theme
latex2web document.tex --theme dark
```

## Themes

**clean-serif** - default, looks like medium
**dark** - dark mode

## test

```bash
cargo run -- tests/sample.tex
open tests/sample.html
```
