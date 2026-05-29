#!/bin/bash
set -x
name="$1"

notes=$(cat <<EOT
See https://gameterm.org/changelog.html#$name for the changelog

If you're looking for nightly downloads or more detailed installation instructions:

[Windows](https://gameterm.org/install/windows.html)
[macOS](https://gameterm.org/install/macos.html)
[Linux](https://gameterm.org/install/linux.html)
[FreeBSD](https://gameterm.org/install/freebsd.html)
EOT
)

gh release view "$name" || gh release create --prerelease --notes "$notes" --title "$name" "$name"
