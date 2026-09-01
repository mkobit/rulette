#!/usr/bin/env bash
set -euo pipefail

readonly package_id="$(cargo pkgid)"
readonly package_version="${package_id##*#}"

invalid_package_version() {
    printf 'could not determine package version from cargo pkgid: %s\n' "${package_id}" >&2
    exit 1
}

validate_identifiers() {
    local value="$1"
    local reject_numeric_leading_zero="$2"
    local identifier
    local -a identifiers

    [[ -n "${value}" && "${value}" != .* && "${value}" != *. && "${value}" != *..* ]] || return 1
    IFS='.' read -r -a identifiers <<< "${value}"
    for identifier in "${identifiers[@]}"; do
        [[ "${identifier}" =~ ^[0-9A-Za-z-]+$ ]] || return 1
        if [[ "${reject_numeric_leading_zero}" == true && "${identifier}" =~ ^0[0-9]+$ ]]; then
            return 1
        fi
    done
}

[[ "${package_id}" != "${package_version}" ]] || invalid_package_version

base_and_prerelease="${package_version}"
build=''
if [[ "${package_version}" == *+* ]]; then
    base_and_prerelease="${package_version%%+*}"
    build="${package_version#*+}"
    [[ -n "${build}" && "${build}" != *+* ]] || invalid_package_version
    validate_identifiers "${build}" false || invalid_package_version
fi

core="${base_and_prerelease}"
prerelease=''
if [[ "${base_and_prerelease}" == *-* ]]; then
    core="${base_and_prerelease%%-*}"
    prerelease="${base_and_prerelease#*-}"
    validate_identifiers "${prerelease}" true || invalid_package_version
fi

[[ -n "${core}" && "${core}" != .* && "${core}" != *. && "${core}" != *..* ]] || invalid_package_version
IFS='.' read -r -a core_identifiers <<< "${core}"
[[ "${#core_identifiers[@]}" -eq 3 ]] || invalid_package_version
for identifier in "${core_identifiers[@]}"; do
    [[ "${identifier}" =~ ^[0-9]+$ && ! ( "${identifier}" =~ ^0[0-9]+$ ) ]] || invalid_package_version
done

printf '%s\n' "${package_version}"
