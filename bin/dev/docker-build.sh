#!/usr/bin/env bash
# Build a local fabro Docker image from the current working tree.
#
# The Docker image uses musl binaries for a small Alpine-based runtime. We
# compile fabro-cli for the target musl triple inside rust:1-bookworm so the
# binary matches what the Alpine runtime expects, regardless of host OS.
# Cargo registry and target dir are cached in named volumes so subsequent
# runs are incremental.
#
# Usage:
#   bin/dev/docker-build.sh                    # host arch, build image at end
#   bin/dev/docker-build.sh --arch amd64       # force amd64 (uses QEMU on non-amd64 hosts — slow)
#   bin/dev/docker-build.sh --arch arm64 --compile-only
#                                              # stage binary only; skip `docker build`
#                                              # (useful for multi-arch buildx verification)

set -euo pipefail

cd "$(dirname "$0")/../.."

arch=""
compile_only=0
while [ $# -gt 0 ]; do
  case "$1" in
    --arch)
      arch="$2"
      shift 2
      ;;
    --compile-only)
      compile_only=1
      shift
      ;;
    *)
      echo "unknown argument: $1" >&2
      exit 1
      ;;
  esac
done

if [ -z "$arch" ]; then
  case "$(uname -m)" in
    x86_64|amd64)   arch=amd64 ;;
    aarch64|arm64)  arch=arm64 ;;
    *) echo "unsupported host arch: $(uname -m)" >&2; exit 1 ;;
  esac
fi

case "$arch" in
  amd64) target=x86_64-unknown-linux-musl  cc_var=CC_x86_64_unknown_linux_musl  linker_var=CARGO_TARGET_X86_64_UNKNOWN_LINUX_MUSL_LINKER ;;
  arm64) target=aarch64-unknown-linux-musl cc_var=CC_aarch64_unknown_linux_musl linker_var=CARGO_TARGET_AARCH64_UNKNOWN_LINUX_MUSL_LINKER ;;
  *) echo "unsupported arch: $arch (expected amd64 or arm64)" >&2; exit 1 ;;
esac

echo "Building fabro-cli for $target inside rust:1-bookworm..."
docker run --rm --platform "linux/$arch" \
  -v "$PWD:/src" \
  -v fabro-docker-cargo-registry:/usr/local/cargo/registry \
  -v "fabro-docker-cargo-target-$arch:/target" \
  -w /src \
  -e CARGO_TARGET_DIR=/target \
  -e "$cc_var=musl-gcc" \
  -e "$linker_var=musl-gcc" \
  -e LIBZ_SYS_STATIC=1 \
  rust:1-bookworm \
  sh -c "
    apt-get update -qq && apt-get install -y -qq musl-tools pkg-config perl make >/dev/null &&
    rustup target add $target &&
    cargo build --release -p fabro-cli --target $target
  "

echo "Extracting binary from builder cache..."
mkdir -p "docker-context/$arch"
docker run --rm --platform "linux/$arch" \
  -v "fabro-docker-cargo-target-$arch:/target" \
  -v "$PWD/docker-context/$arch:/out" \
  rust:1-bookworm \
  cp "/target/$target/release/fabro" /out/fabro

if [ "$compile_only" -eq 1 ]; then
  echo "Staged docker-context/$arch/fabro (skipping docker build per --compile-only)."
  exit 0
fi

echo "Building Docker image..."
docker build --platform "linux/$arch" -t fabro .
