#!/usr/bin/env bash
set -euo pipefail

readonly package_id="$(cargo pkgid)"
readonly package_version="${package_id##*#}"

if [[ "${package_id}" == "${package_version}" ]] || [[ ! "${package_version}" =~ ^(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)(-[0-9A-Za-z-]+(\.[0-9A-Za-z-]+)*)?(\+[0-9A-Za-z-]+(\.[0-9A-Za-z-]+)*)?$ ]]; then
    printf 'could not determine package version from cargo pkgid: %s\n' "${package_id}" >&2
    exit 1
fi

printf '%s\n' "${package_version}"
