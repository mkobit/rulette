#!/usr/bin/env bash
set -euo pipefail

readonly archive_path="${1:?usage: verify-static-release.sh ARCHIVE}"
readonly archive_directory="$(dirname -- "${archive_path}")"
readonly archive_basename="$(basename -- "${archive_path}")"
readonly checksum_path="${archive_path}.sha256"
readonly checksum_basename="$(basename -- "${checksum_path}")"
readonly temp_dir="$(mktemp -d)"
trap 'rm -rf "${temp_dir}"' EXIT

test -f "${archive_path}"
test -f "${checksum_path}"

readonly checksum_line="$(sha256sum -- "${archive_path}")"
test "$(<"${checksum_path}")" = "${checksum_line/"${archive_path}"/"${archive_basename}"}"
(cd "${archive_directory}" && sha256sum --check "${checksum_basename}")

test "$(tar -tzf "${archive_path}")" = 'rulette'
test "$(tar -tvzf "${archive_path}")" = "$(tar -tvzf "${archive_path}" | grep -E '^-.* rulette$')"
tar -xzf "${archive_path}" -C "${temp_dir}" --no-same-owner --no-same-permissions

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
