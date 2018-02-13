#!/usr/bin/env bash

set -euo pipefail

VERSION=$(grep '^version = ' Cargo.toml | sed 's/.*"\([0-9\.]*\)".*/\1/')
GPG_KEY=EA456E8BAF0109429583EED83578F667F2F3A5FA

declare -a targets=(
    "x86_64-musl"
    "i686-musl"
    "armv7-musleabihf"
    "arm-musleabi"
    "arm-musleabihf"
)

declare -a rusttargets=(
    "x86_64-unknown-linux-musl"
    "i686-unknown-linux-musl"
    "armv7-unknown-linux-musleabihf"
    "arm-unknown-linux-musleabi"
    "arm-unknown-linux-musleabihf"
)

function docker-download {
    echo "==> Downloading Docker image: messense/rust-musl-cross:$1"
    docker pull messense/rust-musl-cross:$1
}

function docker-build {
    echo "==> Building target: $1"
    docker run --rm -it -v "$(pwd)":/home/rust/src messense/rust-musl-cross:$1 cargo build --release
}

function copy {
    cp target/
}

echo -e "==> Version $VERSION\n"

for target in ${targets[@]}; do docker-download $target; done
echo ""
for target in ${targets[@]}; do docker-build $target; done
echo ""

rm -rf "dist-$VERSION"
mkdir "dist-$VERSION"

for i in ${!targets[@]}; do
    echo "==> Copying ${targets[$i]}"
    cp "target/${rusttargets[$i]}/release/tldr" "dist-$VERSION/tldr-${targets[$i]}"
done
echo ""

# TODO: Strip binaries, see https://github.com/messense/rust-musl-cross/issues/9

for target in ${targets[@]}; do
    echo "==> Signing $target"
    gpg -a --output "dist-$VERSION/tldr-$target.sig" --detach-sig "dist-$VERSION/tldr-$target"
done
echo ""

echo "Done."
