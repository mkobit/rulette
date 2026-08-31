#!/usr/bin/env bash
set -euo pipefail

readonly target_triple='x86_64-unknown-linux-musl'
readonly binary_name='rulette'
readonly dist_dir='dist'
readonly staging_dir="${dist_dir}/package-root"
readonly release_version="$(cargo pkgid | sed -E 's/.*@//')"
readonly archive_name="rulette-v${release_version}-x86_64-unknown-linux-musl.tar.gz"
readonly archive_path="${dist_dir}/${archive_name}"

rm -rf "${staging_dir}"
mkdir -p "${staging_dir}" "${dist_dir}"
cargo build --locked --release --target x86_64-unknown-linux-musl
install -m 0755 "target/${target_triple}/release/${binary_name}" "${staging_dir}/${binary_name}"
tar --sort=name --mtime=@0 --owner=0 --group=0 --numeric-owner -C "${staging_dir}" -cf - "${binary_name}" | gzip -n > "${archive_path}"
sha256sum "${archive_path}" > "${archive_path}.sha256"
rm -rf "${staging_dir}"
