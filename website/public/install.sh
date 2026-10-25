#!/bin/sh
# portctl universal standalone installer
# https://github.com/tackelua/portctl
# https://portctl.githsoft.com
#
# Usage:
#   curl -fsSL https://portctl.githsoft.com/install.sh | sh
#   or:
#   curl -fsSL https://raw.githubusercontent.com/tackelua/portctl/main/install.sh | sh
#
set -e

# Colors for terminal output
RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
BOLD='\033[1m'
NC='\033[0m' # No Color

log_info() {
    printf "${BLUE}==>${NC} ${BOLD}%s${NC}\n" "$1"
}

log_success() {
    printf "${GREEN}==>${NC} ${BOLD}%s${NC}\n" "$1"
}

log_error() {
    printf "${RED}Error:${NC} %s\n" "$1" >&2
}

main() {
    REPO="tackelua/portctl"

    # 1. Detect OS & Architecture
    OS="$(uname -s)"
    ARCH="$(uname -m)"

    case "$OS" in
        Darwin)
            case "$ARCH" in
                arm64|aarch64)
                    TARGET="aarch64-apple-darwin"
                    ;;
                x86_64|amd64)
                    TARGET="x86_64-apple-darwin"
                    ;;
                *)
                    log_error "Unsupported macOS architecture: $ARCH"
                    exit 1
                    ;;
            esac
            ;;
        Linux)
            case "$ARCH" in
                x86_64|amd64)
                    TARGET="x86_64-unknown-linux-gnu"
                    ;;
                *)
                    log_error "Unsupported Linux architecture: $ARCH (currently only x86_64 is pre-built)"
                    exit 1
                    ;;
            esac
            ;;
        *)
            log_error "Unsupported operating system: $OS. On Windows, please use PowerShell: irm https://portctl.githsoft.com/install.ps1 | iex"
            exit 1
            ;;
    esac

    # 2. Determine Version to Install
    if [ -n "$PORTCTL_VERSION" ]; then
        TAG="$PORTCTL_VERSION"
        case "$TAG" in
            v*) ;;
            *) TAG="v$TAG" ;;
        esac
    else
        log_info "Querying latest release from GitHub..."
        if command -v curl >/dev/null 2>&1; then
            TAG=$(curl -sSL "https://api.github.com/repos/${REPO}/releases/latest" 2>/dev/null | grep '"tag_name":' | head -n 1 | sed -E 's/.*"([^"]+)".*/\1/')
        elif command -v wget >/dev/null 2>&1; then
            TAG=$(wget -qO- "https://api.github.com/repos/${REPO}/releases/latest" 2>/dev/null | grep '"tag_name":' | head -n 1 | sed -E 's/.*"([^"]+)".*/\1/')
        fi

        if [ -z "$TAG" ]; then
            TAG="v2.1.0"
        fi
    fi

    ARCHIVE_NAME="portctl-${TAG}-${TARGET}.tar.gz"
    DOWNLOAD_URL="https://github.com/${REPO}/releases/download/${TAG}/${ARCHIVE_NAME}"
    CHECKSUM_URL="${DOWNLOAD_URL}.sha256"

    log_info "Installing portctl ${TAG} for ${TARGET}..."

    # 3. Download & Unpack in temporary directory
    TMP_DIR=$(mktemp -d 2>/dev/null || mktemp -d -t 'portctl-install')
    if [ -z "$TMP_DIR" ] || [ ! -d "$TMP_DIR" ]; then
        log_error "Failed to create temporary directory."
        exit 1
    fi

    cleanup() {
        if [ -n "$TMP_DIR" ] && [ -d "$TMP_DIR" ]; then
            rm -rf "$TMP_DIR"
        fi
    }
    trap cleanup EXIT
    trap 'exit 130' INT
    trap 'exit 143' TERM

    cd "$TMP_DIR"

    if command -v curl >/dev/null 2>&1; then
        curl -fsSL "$DOWNLOAD_URL" -o "$ARCHIVE_NAME"
        curl -fsSL "$CHECKSUM_URL" -o "${ARCHIVE_NAME}.sha256" 2>/dev/null || true
    elif command -v wget >/dev/null 2>&1; then
        wget -q "$DOWNLOAD_URL" -O "$ARCHIVE_NAME"
        wget -q "$CHECKSUM_URL" -O "${ARCHIVE_NAME}.sha256" 2>/dev/null || true
    else
        log_error "Neither curl nor wget found in PATH."
        exit 1
    fi

    # 4. Checksum verification
    if [ -s "${ARCHIVE_NAME}.sha256" ]; then
        if command -v shasum >/dev/null 2>&1 || command -v sha256sum >/dev/null 2>&1; then
            log_info "Verifying SHA256 checksum..."
            VERIFIED=0
            if command -v shasum >/dev/null 2>&1; then
                if shasum -a 256 -c "${ARCHIVE_NAME}.sha256" >/dev/null 2>&1; then
                    VERIFIED=1
                fi
            elif command -v sha256sum >/dev/null 2>&1; then
                if sha256sum -c "${ARCHIVE_NAME}.sha256" >/dev/null 2>&1; then
                    VERIFIED=1
                fi
            fi

            if [ "$VERIFIED" -eq 1 ]; then
                log_success "Checksum verified."
            else
                log_error "SHA256 checksum verification failed! Download may be corrupted or compromised."
                exit 1
            fi
        else
            log_info "Neither shasum nor sha256sum found, skipping checksum verification."
        fi
    fi

    tar -xzf "$ARCHIVE_NAME"

    if [ ! -f "portctl" ]; then
        log_error "Archive did not contain portctl binary."
        exit 1
    fi

    chmod +x portctl

    # 5. Determine Target Directory
    if [ -n "$PORTCTL_INSTALL_DIR" ]; then
        INSTALL_DIR="$PORTCTL_INSTALL_DIR"
    elif [ -w "/usr/local/bin" ]; then
        INSTALL_DIR="/usr/local/bin"
    elif [ -d "$HOME/.local/bin" ] && echo "$PATH" | grep -q "$HOME/.local/bin"; then
        INSTALL_DIR="$HOME/.local/bin"
    elif [ -d "$HOME/.cargo/bin" ] && echo "$PATH" | grep -q "$HOME/.cargo/bin"; then
        INSTALL_DIR="$HOME/.cargo/bin"
    else
        INSTALL_DIR="/usr/local/bin"
    fi

    # 6. Install Binary
    if [ -w "$INSTALL_DIR" ]; then
        mkdir -p "$INSTALL_DIR"
        cp portctl "$INSTALL_DIR/portctl"
        chmod 755 "$INSTALL_DIR/portctl"
    else
        log_info "Elevated permissions required to install to $INSTALL_DIR:"
        if command -v sudo >/dev/null 2>&1; then
            sudo mkdir -p "$INSTALL_DIR"
            sudo cp portctl "$INSTALL_DIR/portctl"
            sudo chmod 755 "$INSTALL_DIR/portctl"
        else
            log_error "Cannot write to $INSTALL_DIR and sudo is not available. Set PORTCTL_INSTALL_DIR to a writable directory."
            exit 1
        fi
    fi

    # 7. Verify Installation
    if [ -x "$INSTALL_DIR/portctl" ]; then
        VERSION_OUTPUT=$("$INSTALL_DIR/portctl" --version 2>&1 || true)
        log_success "portctl successfully installed to $INSTALL_DIR/portctl ($VERSION_OUTPUT)!"
    else
        log_success "portctl binary placed in $INSTALL_DIR/portctl."
    fi

    # Check if INSTALL_DIR is in PATH
    case ":$PATH:" in
        *":$INSTALL_DIR:"*) ;;
        *)
            printf "${BLUE}Notice:${NC} %s is not in your \$PATH. You may need to add it:\n" "$INSTALL_DIR"
            printf "  export PATH=\"%s:\$PATH\"\n" "$INSTALL_DIR"
            ;;
    esac
}

main "$@"
