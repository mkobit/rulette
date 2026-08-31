#!/usr/bin/env bash
set -euo pipefail

readonly archive_path="${1:?usage: verify-static-release.sh ARCHIVE}"
readonly archive_basename="$(basename -- "${archive_path}")"
readonly checksum_path="${archive_path}.sha256"
readonly temp_dir="$(mktemp -d)"
trap 'rm -rf "${temp_dir}"' EXIT
readonly snapshot_archive="${temp_dir}/${archive_basename}"
readonly snapshot_checksum="${snapshot_archive}.sha256"
readonly snapshot_checksum_basename="$(basename -- "${snapshot_checksum}")"

test -f "${archive_path}"
test -f "${checksum_path}"
cp -- "${archive_path}" "${snapshot_archive}"
cp -- "${checksum_path}" "${snapshot_checksum}"

readonly checksum_hash="$(sha256sum -- "${snapshot_archive}" | awk '{print $1}')"
test "$(<"${snapshot_checksum}")" = "${checksum_hash}  ${archive_basename}"
(cd "${temp_dir}" && sha256sum --check -- "${snapshot_checksum_basename}")

test "$(tar -tzf "${snapshot_archive}")" = 'rulette'
test "$(tar -tvzf "${snapshot_archive}")" = "$(tar -tvzf "${snapshot_archive}" | grep -E '^-.* rulette$')"
tar -xzf "${snapshot_archive}" -C "${temp_dir}" --no-same-owner --no-same-permissions

readonly binary_path="${temp_dir}/rulette"
test -f "${binary_path}"
test ! -L "${binary_path}"
test -x "${binary_path}"
file --brief "${binary_path}" | grep -Eq '^ELF .*statically linked'
readonly dynamic_output="$(readelf --dynamic "${binary_path}")"
if grep -q 'NEEDED' <<<"${dynamic_output}"; then
    exit 1
fi
readonly ldd_output="$(ldd "${binary_path}" 2>&1 || true)"
grep -Eq 'not a dynamic executable|statically linked' <<<"${ldd_output}"
(cd "${temp_dir}" && env -i PATH="${PATH}" ./rulette --version)
(cd "${temp_dir}" && env -i PATH="${PATH}" ./rulette schema --to graph >/dev/null)
