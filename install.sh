#!/bin/sh
set -e

# ghui installer script
# Installs the latest release of ghui for macOS and Linux
# Works with bash, zsh, fish, and other POSIX-compatible shells

REPO="abeljim8am/ghui"
INSTALL_DIR="${INSTALL_DIR:-$HOME/.local/bin}"
BINARY_NAME="ghui"

# Colors for output (will be disabled if terminal doesn't support them)
if [ -t 1 ]; then
    RED='\033[0;31m'
    GREEN='\033[0;32m'
    YELLOW='\033[1;33m'
    NC='\033[0m'
else
    RED=''
    GREEN=''
    YELLOW=''
    NC=''
fi

info() {
    printf "${GREEN}[INFO]${NC} %s\n" "$1"
}

warn() {
    printf "${YELLOW}[WARN]${NC} %s\n" "$1"
}

error() {
    printf "${RED}[ERROR]${NC} %s\n" "$1"
    exit 1
}

# Detect OS and architecture
detect_platform() {
    os="$(uname -s)"
    arch="$(uname -m)"

    case "$os" in
        Darwin)
            case "$arch" in
                arm64|aarch64)
                    printf "macos-arm64"
                    ;;
                x86_64)
                    error "macOS x86_64 is not currently supported. Only ARM64 (Apple Silicon) is available."
                    ;;
                *)
                    error "Unsupported macOS architecture: $arch"
                    ;;
            esac
            ;;
        Linux)
            case "$arch" in
                x86_64)
                    printf "linux-x64"
                    ;;
                aarch64|arm64)
                    error "Linux ARM64 is not currently supported. Only x86_64 is available."
                    ;;
                *)
                    error "Unsupported Linux architecture: $arch"
                    ;;
            esac
            ;;
        *)
            error "Unsupported operating system: $os. This installer supports macOS and Linux only."
            ;;
    esac
}

# Get the latest release version from GitHub
get_latest_version() {
    if command -v curl > /dev/null 2>&1; then
        version=$(curl -sL "https://api.github.com/repos/${REPO}/releases/latest" | grep '"tag_name":' | sed -E 's/.*"([^"]+)".*/\1/')
    elif command -v wget > /dev/null 2>&1; then
        version=$(wget -qO- "https://api.github.com/repos/${REPO}/releases/latest" | grep '"tag_name":' | sed -E 's/.*"([^"]+)".*/\1/')
    else
        error "Neither curl nor wget found. Please install one of them."
    fi

    if [ -z "$version" ]; then
        error "Failed to fetch the latest version. Please check your internet connection or try again later."
    fi

    printf "%s" "$version"
}

# Download and install the binary
install_ghui() {
    platform=$(detect_platform)
    info "Detected platform: $platform"

    info "Fetching latest version..."
    version=$(get_latest_version)
    info "Latest version: $version"

    download_url="https://github.com/${REPO}/releases/download/${version}/${BINARY_NAME}-${platform}"

    info "Downloading ${BINARY_NAME} ${version}..."

    tmp_dir=$(mktemp -d)
    trap 'rm -rf "$tmp_dir"' EXIT

    if command -v curl > /dev/null 2>&1; then
        curl -sL "$download_url" -o "${tmp_dir}/${BINARY_NAME}"
    elif command -v wget > /dev/null 2>&1; then
        wget -q "$download_url" -O "${tmp_dir}/${BINARY_NAME}"
    fi

    if [ ! -f "${tmp_dir}/${BINARY_NAME}" ] || [ ! -s "${tmp_dir}/${BINARY_NAME}" ]; then
        error "Download failed. Please check your internet connection or try again later."
    fi

    chmod +x "${tmp_dir}/${BINARY_NAME}"

    info "Installing to ${INSTALL_DIR}..."

    # Create install directory if it doesn't exist
    if [ ! -d "$INSTALL_DIR" ]; then
        mkdir -p "$INSTALL_DIR"
    fi

    # Check if we need sudo
    if [ -w "$INSTALL_DIR" ]; then
        mv "${tmp_dir}/${BINARY_NAME}" "${INSTALL_DIR}/${BINARY_NAME}"
    else
        warn "Requesting sudo access to install to ${INSTALL_DIR}"
        sudo mv "${tmp_dir}/${BINARY_NAME}" "${INSTALL_DIR}/${BINARY_NAME}"
    fi

    info "Successfully installed ${BINARY_NAME} ${version} to ${INSTALL_DIR}/${BINARY_NAME}"

    # Verify installation
    if command -v "$BINARY_NAME" > /dev/null 2>&1; then
        info "Installation verified. Run '${BINARY_NAME}' to get started!"
    else
        warn "Installation complete, but '${BINARY_NAME}' is not in your PATH."
        warn "Add ${INSTALL_DIR} to your PATH or run: ${INSTALL_DIR}/${BINARY_NAME}"
    fi
}

# Main
main() {
    printf "\n"
    printf "  ╔═══════════════════════════════════════╗\n"
    printf "  ║         ghui installer                ║\n"
    printf "  ║   GitHub PR TUI for your terminal     ║\n"
    printf "  ╚═══════════════════════════════════════╝\n"
    printf "\n"

    install_ghui
}

main "$@"
