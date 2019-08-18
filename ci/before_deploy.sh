#!/bin/bash
set -eux

build() {
  cargo build --target "$TARGET" --release --verbose
}

pack() {
  local -r pwd=$(pwd)
  local -r tempdir=$(mktemp -d 2>/dev/null || mktemp -d -t tmp)
  local -r package_name="$PROJECT_NAME-$TRAVIS_TAG-$TARGET"
  mkdir "$tempdir/$package_name"
  cp "target/$TARGET/release/$PROJECT_NAME" README.md LICENSE_1_0.txt \
    "$tempdir/$package_name"
  pushd "$tempdir"
  tar cJf "$pwd/$package_name.tar.xz" "$package_name"
  popd
  rm -rf "$tempdir"
}

main() {
  build
  pack
}

main
