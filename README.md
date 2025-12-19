# ghui

A terminal user interface (TUI) for viewing and managing GitHub pull requests.

## Features

- **Three PR Views**:
  - My PRs: Pull requests you've authored
  - Review Requested: PRs where your review is requested
  - Labels: PRs matching configured labels

- **Fuzzy Search**: Quickly filter PRs using fuzzy matching
- **CI Status**: View pass/fail/pending status at a glance
- **Branch Checkout**: Checkout PR branches directly (supports both git and jj)
- **Labels Management**: Configure repo-specific or global label filters
- **Caching**: SQLite-based caching for fast startup

## Requirements

- [GitHub CLI](https://cli.github.com/) (`gh`) installed and authenticated
- Also need to make sure `git` is installed

## Installation

### Quick Install (Recommended)

Install the latest release with a single command (works with bash, zsh, fish, and other shells):

```bash
curl -fsSL https://raw.githubusercontent.com/abeljim8am/ghui/main/install.sh | sh
```

Or with wget:

```bash
wget -qO- https://raw.githubusercontent.com/abeljim8am/ghui/main/install.sh | sh
```

To install to a custom directory (e.g., `/usr/local/bin`):

```bash
curl -fsSL https://raw.githubusercontent.com/abeljim8am/ghui/main/install.sh | INSTALL_DIR=/usr/local/bin sh
```

### From Source

```bash
cargo install --path .
```

### Pre-built Binaries

Download from the [Releases](https://github.com/abeljim8am/ghui/releases) page.

## Usage

Run `ghui` from within a Git repository:

```bash
cd your-repo
ghui
```

### Keybindings

| Key | Action |
|-----|--------|
| `1` | Switch to My PRs tab |
| `2` | Switch to Review Requested tab |
| `3` | Switch to Labels tab |
| `j` / `↓` | Move down |
| `k` / `↑` | Move up |
| `g` | Go to top |
| `G` | Go to bottom |
| `/` | Start fuzzy search |
| `Enter` / `o` | Open PR in browser |
| `c` | Checkout branch |
| `r` | Refresh current view |
| `l` | Manage labels |
| `?` | Show help |
| `q` | Quit |

### Search Mode

| Key | Action |
|-----|--------|
| Type | Filter PRs |
| `Enter` | Accept search, exit search mode |
| `Esc` | Clear search and exit |
| `↓` / `Tab` | Move to next result |
| `↑` / `Shift+Tab` | Move to previous result |

### Labels Management

Press `l` to open the labels popup:

| Key | Action |
|-----|--------|
| `a` | Add new label |
| `d` | Delete selected label |
| `j` / `↓` | Move down |
| `k` / `↑` | Move up |
| `Esc` | Close popup |

Labels can be scoped to the current repository or set as global (applies to all repos).

## Configuration

ghui stores its cache and configuration in:
- macOS: `~/Library/Application Support/ghui/`
- Linux: `~/.config/ghui/`
- Windows: `%APPDATA%\ghui\`

## Building from Source

```bash
# Clone the repository
git clone https://github.com/yourusername/ghui.git
cd ghui

# Build release binary
cargo build --release

# The binary will be at ./target/release/ghui
```

### Development

```bash
# Run with cargo
cargo run

# Run clippy
cargo clippy -- -D warnings

# Format code
cargo fmt
```

## Architecture

ghui uses a Model-View-Update (MVU/Elm) architecture:

- **Model** (`src/app/model.rs`): Application state
- **Message** (`src/app/message.rs`): All possible events/actions
- **Update** (`src/app/update.rs`): State transitions based on messages
- **View** (`src/view/`): UI rendering components
