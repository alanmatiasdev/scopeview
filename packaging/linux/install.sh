#!/bin/sh
# Install ScopeView from the release tarball.
#   ./install.sh                  per-user install into ~/.local
#   sudo ./install.sh /usr/local  system-wide install (any prefix works)
#   ./install.sh --uninstall [prefix]
set -eu

ID=io.github.alanmatiasdev.ScopeView
here=$(cd "$(dirname "$0")" && pwd)

uninstall=0
if [ "${1:-}" = "--uninstall" ]; then
    uninstall=1
    shift
fi
prefix=${1:-$HOME/.local}

if [ "$uninstall" = 1 ]; then
    rm -f "$prefix/bin/scopeview" \
        "$prefix/share/applications/$ID.desktop" \
        "$prefix/share/metainfo/$ID.metainfo.xml" \
        "$prefix/share/icons/hicolor/scalable/apps/$ID.svg" \
        "$prefix/share/icons/hicolor/256x256/apps/$ID.png"
    echo "ScopeView removed from $prefix"
    exit 0
fi

install -Dm755 "$here/scopeview" "$prefix/bin/scopeview"
install -Dm644 "$here/share/$ID.desktop" "$prefix/share/applications/$ID.desktop"
install -Dm644 "$here/share/$ID.metainfo.xml" "$prefix/share/metainfo/$ID.metainfo.xml"
install -Dm644 "$here/share/scopeview.svg" "$prefix/share/icons/hicolor/scalable/apps/$ID.svg"
install -Dm644 "$here/share/$ID.png" "$prefix/share/icons/hicolor/256x256/apps/$ID.png"

echo "ScopeView installed in $prefix"
echo "To open /dev/video* your user must be in the 'video' group: sudo usermod -aG video \$USER"
case ":$PATH:" in
    *":$prefix/bin:"*) ;;
    *) echo "Note: $prefix/bin is not in your PATH" ;;
esac
