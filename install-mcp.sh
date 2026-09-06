#!/usr/bin/env bash
set -eo pipefail

# bl1nk-kept MCP Server installer
# Usage: curl -fsSL https://raw.githubusercontent.com/billlzzz18/bl1nk-kept/master/install-mcp.sh | bash

REPO="billlzzz18/bl1nk-kept"
BINARY_NAME="bl1nk-kept-mcp"
INSTALL_DIR="${KEPT_MCP_INSTALL_DIR:-$HOME/.local/bin}"

info() { printf '\033[1;34m%s\033[0m\n' "$*"; }
success() { printf '\033[1;38;5;208m%s\033[0m\n' "$*"; }
warn() { printf '\033[1;33m%s\033[0m\n' "$*"; }
error() { printf '\033[1;31mError: %s\033[0m\n' "$*" >&2; exit 1; }

print_json() {
    if command -v jq &>/dev/null; then jq . <<<"$1"; else printf '%s\n' "$1"; fi
}

detect_platform() {
    local os arch
    os="$(uname -s)"
    arch="$(uname -m)"
    case "$os" in
        Linux) case "$arch" in x86_64) echo "x86_64-unknown-linux-gnu" ;; aarch64|arm64) echo "aarch64-unknown-linux-gnu" ;; *) error "Unsupported architecture: $arch" ;; esac ;;
        Darwin) case "$arch" in x86_64) echo "x86_64-apple-darwin" ;; arm64|aarch64) echo "aarch64-apple-darwin" ;; *) error "Unsupported architecture: $arch" ;; esac ;;
        MINGW*|MSYS*|CYGWIN*) case "$arch" in x86_64) echo "x86_64-pc-windows-msvc" ;; aarch64|arm64) echo "aarch64-pc-windows-msvc" ;; *) error "Unsupported architecture: $arch" ;; esac ;;
        *) error "Unsupported OS: $os" ;;
    esac
}

get_release() {
    local target="$1" releases_json
    releases_json="$(curl -fsSL "https://api.github.com/repos/${REPO}/releases")" \
        || error "Failed to fetch releases from https://github.com/${REPO}/releases"
    if command -v jq &>/dev/null; then
        jq -r --arg asset "${BINARY_NAME}-${target}" '.[] | select(.draft == false) | select(any(.assets[]?; .name == $asset or .name == ($asset + ".exe"))) | .tag_name' <<<"$releases_json" | head -n 1
    else
        grep -oE '"tag_name": *"[^"]+"' <<<"$releases_json" | head -n 1 | cut -d'"' -f4
    fi
}

install_binary() {
    local target="$1" tag="$2" ext=""
    [[ "$target" == *windows* ]] && ext=".exe"
    local filename="${BINARY_NAME}-${target}${ext}"
    local base_url="https://github.com/${REPO}/releases/download/${tag}"
    local tmp_dir
    tmp_dir="$(mktemp -d)"
    trap 'rm -rf "$tmp_dir"' EXIT

    info "Downloading ${filename} from release ${tag}..."
    curl -fsSL -o "${tmp_dir}/${filename}" "${base_url}/${filename}" \
        || error "Failed to download ${filename} from ${base_url}"
    if command -v sha256sum &>/dev/null && curl -fsSL -o "${tmp_dir}/${filename}.sha256" "${base_url}/${filename}.sha256"; then
        info "Verifying checksum..."
        (cd "$tmp_dir" && sha256sum -c "${filename}.sha256") || error "Checksum verification failed"
    elif command -v shasum &>/dev/null && curl -fsSL -o "${tmp_dir}/${filename}.sha256" "${base_url}/${filename}.sha256"; then
        info "Verifying checksum..."
        expected="$(awk '{print $1}' "${tmp_dir}/${filename}.sha256")"
        actual="$(shasum -a 256 "${tmp_dir}/${filename}" | awk '{print $1}')"
        [[ "$expected" == "$actual" ]] || error "Checksum verification failed"
    else
        warn "Checksum file unavailable; skipping verification."
    fi

    mkdir -p "$INSTALL_DIR"
    install -m 0755 "${tmp_dir}/${filename}" "${INSTALL_DIR}/${BINARY_NAME}${ext}"
}

check_path() {
    case ":$PATH:" in *":${INSTALL_DIR}:"*) return 0 ;; esac
    warn "${INSTALL_DIR} is not in your PATH."
    echo "  echo 'export PATH=\"${INSTALL_DIR}:\$PATH\"' >> ~/.bashrc"
    echo "  source ~/.bashrc"
}

main() {
    local target tag
    target="$(detect_platform)"
    tag="$(get_release "$target")"
    [[ -n "$tag" ]] || error "No release contains ${BINARY_NAME} for ${target}"
    install_binary "$target" "$tag"
    success "Installed ${BINARY_NAME} ${tag} to ${INSTALL_DIR}/${BINARY_NAME}"
    check_path
    echo ""
    echo "Claude Code: claude mcp add -s user kept -- ${INSTALL_DIR}/${BINARY_NAME}"
    echo "Binary:      ${INSTALL_DIR}/${BINARY_NAME}"
    echo "Docs:        https://github.com/${REPO}"
}

main "$@"
