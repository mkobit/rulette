#!/usr/bin/env bash
set -euo pipefail

readonly target_triple='x86_64-unknown-linux-musl'
readonly binary_name='rulette'
readonly dist_dir='dist'
readonly script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
readonly repository_root="$(cd "${script_dir}/.." && pwd)"

cd "${repository_root}"
readonly release_version="$(cargo pkgid | sed -E 's/.*@//')"
readonly archive_name="rulette-v${release_version}-x86_64-unknown-linux-musl.tar.gz"
readonly archive_path="${dist_dir}/${archive_name}"
mkdir -p "${dist_dir}"
readonly staging_dir="$(mktemp -d "${dist_dir}/package-root.XXXXXX")"
trap 'rm -rf -- "${staging_dir}"' EXIT
cargo build --locked --release --target x86_64-unknown-linux-musl
install -m 0755 "target/${target_triple}/release/${binary_name}" "${staging_dir}/${binary_name}"
tar --sort=name --mtime=@0 --owner=0 --group=0 --numeric-owner -C "${staging_dir}" -cf - "${binary_name}" | gzip -n > "${archive_path}"
(cd "${dist_dir}" && sha256sum "${archive_name}" > "${archive_name}.sha256")
