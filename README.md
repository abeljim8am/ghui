# ghui

A terminal user interface (TUI) for viewing and managing GitHub pull requests.

## Features

- **Three PR Views**:
  - My PRs: Pull requests you've authored
  - Review Requested: PRs where your review is requested
  - Labels: PRs matching configured labels

- **CI Integration**:
  - View CI status (pass/fail/pending) at a glance
  - Workflows view showing all CI checks (GitHub Actions, CircleCI, etc.)
  - Job logs with foldable steps
  - Test failure extraction and copy-to-clipboard
  - Annotations view for reviewdog and similar tools

- **PR Preview**: View PR description, comments, and reviews in-terminal with markdown rendering

- **Fuzzy Search**: Quickly filter PRs using fuzzy matching

- **Branch Checkout**: Checkout PR branches directly (supports both git and jujutsu)

- **Labels Management**: Configure repo-specific or global label filters

- **Caching**: SQLite-based caching for fast startup with auto-refresh every 30 seconds

## Requirements

- [GitHub CLI](https://cli.github.com/) (`gh`) installed and authenticated
- Git (or [jujutsu](https://github.com/martinvonz/jj) for jj-based repos)

## Environment Variables

| Variable | Required | Description |
|----------|----------|-------------|
| `GH_TOKEN` | No | GitHub personal access token. If not set, falls back to `gh auth token` (requires GitHub CLI to be authenticated) |
| `CIRCLECI_TOKEN` | No | CircleCI API token for viewing CircleCI job logs. Required only if your project uses CircleCI |
| `EDITOR` | No | Preferred text editor for viewing job logs (e.g., `vim`, `nvim`, `code`). Falls back to `VISUAL`, then `vim` |
| `VISUAL` | No | Alternative to `EDITOR` for graphical editors |

### Setting Up Environment Variables

**For GitHub authentication**, you have two options:

1. **Use GitHub CLI (recommended)**: Simply run `gh auth login` and ghui will automatically use your token
2. **Use GH_TOKEN**: Set the environment variable with a personal access token

```bash
# Option 1: Use GitHub CLI (no env var needed)
gh auth login

# Option 2: Set GH_TOKEN in your shell profile
export GH_TOKEN="ghp_your_token_here"
```

**For CircleCI integration** (optional):

```bash
# Add to your shell profile (~/.bashrc, ~/.zshrc, etc.)
export CIRCLECI_TOKEN="your_circleci_token_here"
```

To generate a CircleCI token:
1. Go to [CircleCI Personal API Tokens](https://app.circleci.com/settings/user/tokens)
2. Click "Create New Token"
3. Give it a name and copy the token

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

**Supported platforms**:
- macOS ARM64 (Apple Silicon)
- Linux x64
- Linux ARM64

## Usage

Run `ghui` from within a Git repository:

```bash
cd your-repo
ghui
```

### Command Line Options

| Option | Description |
|--------|-------------|
| `-v`, `--version` | Print version |
| `--clear-cache` | Clear the local cache and exit |

### Keybindings

#### Main View

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
| `Enter` | Open PR preview |
| `o` | Open PR in browser |
| `c` | Checkout branch |
| `w` | Open workflows/CI view |
| `p` | Open PR preview |
| `r` | Refresh current view |
| `l` | Manage labels |
| `?` | Show help |
| `q` | Quit |

#### Search Mode

| Key | Action |
|-----|--------|
| Type | Filter PRs |
| `Enter` | Accept search, exit search mode |
| `Esc` | Clear search and exit |
| `↓` / `Tab` | Move to next result |
| `↑` / `Shift+Tab` | Move to previous result |

#### PR Preview View

| Key | Action |
|-----|--------|
| `j` / `↓` | Scroll down |
| `k` / `↑` | Scroll up |
| `Ctrl+d` | Half-page down |
| `Ctrl+u` | Half-page up |
| `g` | Go to top |
| `G` | Go to bottom |
| `o` | Open PR in browser |
| `q` / `Esc` | Close preview |

#### Workflows View

| Key | Action |
|-----|--------|
| `j` / `↓` | Next job |
| `k` / `↑` | Previous job |
| `Enter` | Open job logs |
| `r` | Refresh CI status |
| `o` | Open in browser |
| `q` / `Esc` | Close workflows view |

#### Job Logs View

| Key | Action |
|-----|--------|
| `j` / `↓` | Next step / scroll down |
| `k` / `↑` | Previous step / scroll up |
| `Space` | Toggle step expansion |
| `Enter` | Open step in external editor |
| `y` | Copy test failures |
| `x` | Copy full step output |
| `o` | Open in browser |
| `q` / `Esc` | Close job logs |

#### Annotations View

| Key | Action |
|-----|--------|
| `j` / `↓` | Next annotation |
| `k` / `↑` | Previous annotation |
| `v` / `Space` | Toggle annotation selection |
| `y` | Copy selected annotations |
| `o` | Open in browser |
| `q` / `Esc` | Close annotations |

#### Labels Management

Press `l` to open the labels popup:

| Key | Action |
|-----|--------|
| `a` | Add new label |
| `d` | Delete selected label |
| `j` / `↓` | Move down |
| `k` / `↑` | Move up |
| `Esc` | Close popup |

When adding a label, press `Tab` to toggle between repo-specific and global scope.

## Configuration

ghui stores its cache and configuration in:
- macOS: `~/Library/Application Support/ghui/`
- Linux: `~/.config/ghui/`
- Windows: `%APPDATA%\ghui\`

The cache is stored in a SQLite database (`cache.db`) and includes:
- Cached PR data for fast startup
- Configured label filters (repo-specific and global)

Use `ghui --clear-cache` to reset the cache if needed.

## Building from Source

```bash
# Clone the repository
git clone https://github.com/abeljim8am/ghui.git
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

### Services

- `src/services/github.rs`: GitHub API integration (PRs, Actions, job logs)
- `src/services/circleci.rs`: CircleCI API integration
- `src/services/cache.rs`: SQLite caching layer
- `src/services/search.rs`: Fuzzy search implementation

### Version Control Support

ghui automatically detects whether you're in a git or jujutsu repository by checking for a `.jj` directory.

**Repository detection:**
- Git repos: Reads remote URL via `git remote get-url origin`
- Jujutsu repos: Reads remote URL via `jj git remote list`

**Branch checkout behavior:**

| VCS | Command | Fallback |
|-----|---------|----------|
| Git | `git switch <branch>` | - |
| Jujutsu | `jj edit <branch>@origin` | `jj new <branch>@origin` |

For jujutsu, `edit` is attempted first to move the working copy to the commit. If that fails (e.g., the commit is immutable), it falls back to `new` which creates a new mutable working copy change on top of the remote branch.

## License

MIT
