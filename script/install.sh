#!/usr/bin/env sh
set -eu

# Downloads a tarball from https://xenomorphic.dev/releases and unpacks it
# into ~/.local/. If you'd prefer to do this manually, instructions are at
# https://xenomorphic.dev/docs/linux.

main() {
    platform="$(uname -s)"
    arch="$(uname -m)"
    channel="${XENOMORPHIC_CHANNEL:-stable}"
    XENOMORPHIC_VERSION="${XENOMORPHIC_VERSION:-latest}"
    # Use TMPDIR if available (for environments with non-standard temp directories)
    if [ -n "${TMPDIR:-}" ] && [ -d "${TMPDIR}" ]; then
        temp="$(mktemp -d "$TMPDIR/xenomorphic-XXXXXX")"
    else
        temp="$(mktemp -d "/tmp/xenomorphic-XXXXXX")"
    fi

    if [ "$platform" = "Darwin" ]; then
        platform="macos"
    elif [ "$platform" = "Linux" ]; then
        platform="linux"
    else
        echo "Unsupported platform $platform"
        exit 1
    fi

    case "$platform-$arch" in
        macos-arm64* | linux-arm64* | linux-armhf | linux-aarch64)
            arch="aarch64"
            ;;
        macos-x86* | linux-x86* | linux-i686*)
            arch="x86_64"
            ;;
        *)
            echo "Unsupported platform or architecture"
            exit 1
            ;;
    esac

    if command -v curl >/dev/null 2>&1; then
        curl () {
            command curl -fL "$@"
        }
    elif command -v wget >/dev/null 2>&1; then
        curl () {
            wget -O- "$@"
        }
    else
        echo "Could not find 'curl' or 'wget' in your path"
        exit 1
    fi

    "$platform" "$@"

    if [ "$(command -v xed)" = "$HOME/.local/bin/xed" ]; then
        echo "Xenomorphic has been installed. Run with 'xed'"
    else
        echo "To run Xenomorphic from your terminal, you must add ~/.local/bin to your PATH"
        echo "Run:"

        case "$SHELL" in
            *zsh)
                echo "   echo 'export PATH=\$HOME/.local/bin:\$PATH' >> ~/.zshrc"
                echo "   source ~/.zshrc"
                ;;
            *fish)
                echo "   fish_add_path -U $HOME/.local/bin"
                ;;
            *)
                echo "   echo 'export PATH=\$HOME/.local/bin:\$PATH' >> ~/.bashrc"
                echo "   source ~/.bashrc"
                ;;
        esac

        echo "To run Xenomorphic now, '~/.local/bin/xed'"
    fi
}

linux() {
    if [ -n "${XENOMORPHIC_BUNDLE_PATH:-}" ]; then
        cp "$XENOMORPHIC_BUNDLE_PATH" "$temp/xenomorphic-linux-$arch.tar.gz"
    else
        echo "Downloading Xenomorphic version: $XENOMORPHIC_VERSION"
        curl "https://cloud.xenomorphic.dev/releases/$channel/$XENOMORPHIC_VERSION/download?asset=xenomorphic&arch=$arch&os=linux&source=install.sh" > "$temp/xenomorphic-linux-$arch.tar.gz"
    fi

    suffix=""
    if [ "$channel" != "stable" ]; then
        suffix="-$channel"
    fi

    appid=""
    case "$channel" in
      stable)
        appid="dev.xenomorphic.Xenomorphic"
        ;;
      nightly)
        appid="dev.xenomorphic.Xenomorphic-Nightly"
        ;;
      preview)
        appid="dev.xenomorphic.Xenomorphic-Preview"
        ;;
      dev)
        appid="dev.xenomorphic.Xenomorphic-Dev"
        ;;
      *)
        echo "Unknown release channel: ${channel}. Using stable app ID."
        appid="dev.xenomorphic.Xenomorphic"
        ;;
    esac

    # Unpack
    rm -rf "$HOME/.local/xenomorphic$suffix.app"
    mkdir -p "$HOME/.local/xenomorphic$suffix.app"
    tar -xzf "$temp/xenomorphic-linux-$arch.tar.gz" -C "$HOME/.local/"

    # Setup ~/.local directories
    mkdir -p "$HOME/.local/bin" "$HOME/.local/share/applications"

    # Link the binary
    ln -sf "$HOME/.local/xenomorphic$suffix.app/bin/cli" "$HOME/.local/bin/xed"

    # Copy .desktop file
    desktop_file_path="$HOME/.local/share/applications/${appid}.desktop"
    src_dir="$HOME/.local/xenomorphic$suffix.app/share/applications"
    if [ -f "$src_dir/${appid}.desktop" ]; then
        cp "$src_dir/${appid}.desktop" "${desktop_file_path}"
    else
        # Fallback for older tarballs
        cp "$src_dir/xenomorphic$suffix.desktop" "${desktop_file_path}"
    fi
    sed -i "s|Icon=xenomorphic|Icon=$HOME/.local/xenomorphic$suffix.app/share/icons/hicolor/512x512/apps/xenomorphic.png|g" "${desktop_file_path}"
    sed -i "s|TryExec=xed|TryExec=$HOME/.local/xenomorphic$suffix.app/bin/cli|g" "${desktop_file_path}"
    sed -i "s|Exec=xed|Exec=$HOME/.local/xenomorphic$suffix.app/bin/cli|g" "${desktop_file_path}"
}

macos() {
    echo "Downloading Xenomorphic version: $XENOMORPHIC_VERSION"
    curl "https://cloud.xenomorphic.dev/releases/$channel/$XENOMORPHIC_VERSION/download?asset=xenomorphic&os=macos&arch=$arch&source=install.sh" > "$temp/Xenomorphic-$arch.dmg"
    hdiutil attach -quiet "$temp/Xenomorphic-$arch.dmg" -mountpoint "$temp/mount"
    app="$(cd "$temp/mount/"; echo *.app)"
    echo "Installing $app"
    if [ -d "/Applications/$app" ]; then
        echo "Removing existing $app"
        rm -rf "/Applications/$app"
    fi
    ditto "$temp/mount/$app" "/Applications/$app"
    hdiutil detach -quiet "$temp/mount"

    mkdir -p "$HOME/.local/bin"
    # Link the binary
    ln -sf "/Applications/$app/Contents/MacOS/cli" "$HOME/.local/bin/xed"
}

main "$@"
