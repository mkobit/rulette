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

    case "${value}" in
        '' | .* | *. | *..*) return 1 ;;
    esac
    IFS='.' read -r -a identifiers <<< "${value}"
    for identifier in "${identifiers[@]}"; do
        case "${identifier}" in
            *[!0-9A-Za-z-]*) return 1 ;;
        esac
        if [[ "${reject_numeric_leading_zero}" == true ]]; then
            case "${identifier}" in
                0[0-9]*) return 1 ;;
            esac
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

case "${core}" in
    '' | .* | *. | *..*) invalid_package_version ;;
esac
IFS='.' read -r -a core_identifiers <<< "${core}"
[[ "${#core_identifiers[@]}" -eq 3 ]] || invalid_package_version
for identifier in "${core_identifiers[@]}"; do
    case "${identifier}" in
        '' | *[!0-9]* | 0[0-9]*) invalid_package_version ;;
    esac
done

printf '%s\n' "${package_version}"
