#!/usr/bin/env bash

if [[ -z $IN_NIX_SHELL ]]; then
	echo "The release script must be run from inside a nix shell"
	echo "    run 'nix develop' first"
    exit 1
fi

if [[ ($# -eq 0) || $1 == "--help" || $1 == "-h" ]]; then
    echo "Usage: $0 [next-major|next-minor|next-patch|<semver>]"
    exit 1
fi

# check if there are any modified files (based on https://stackoverflow.com/a/3879077)
git update-index --refresh &> /dev/null
git diff-index --quiet HEAD --
if [[ ($? -eq 1) ]]; then
    echo "Release script can only run on a clean repo. Please stash your changes."
    exit 1
fi

if [[ $(git rev-parse --abbrev-ref HEAD) != "master" ]]; then
    echo "WARNING: you are trying to cut a release from a branch other than master!"
    read -r -n1 -p "Do you wish to continue? [y/n]" yesno
    printf "\n"
    if [[ "$yesno" =~ ^[^Yy]$ ]]; then
        echo "Aborting..."
        exit 1
    fi
fi

# we first do a dry-run and parse the output to check that all crates are
# on the same version and to get the new version string
case $1 in
    next-major) setversion_result=$(cargo set-version --dry-run --bump major 2>&1) ;;
    next-minor) setversion_result=$(cargo set-version --dry-run --bump minor 2>&1) ;;
    next-patch) setversion_result=$(cargo set-version --dry-run --bump patch 2>&1) ;;
    *) setversion_result=$(cargo set-version --dry-run $1 2>&1) ;;
esac

# if cargo set-version failed we exit
if [ $? -ne 0 ]; then
    echo "Error: cargo set-version failed to upgrade crate versions"
    exit 1
fi

# check that all crates are on the same version
readarray -t current_versions <<<$(echo "$setversion_result" |
    head -n -1 | # we drop the dry-run warning line from the output
    sed -r "s/^\s*Upgrading \S+ from (\S+) to \S+$/\1/g")

if [ $(printf "%s\000" "${current_versions[@]}" |
       LC_ALL=C sort -z -u |
       grep -z -c .) -ne 1 ] ; then
    echo "ERROR: Not all crates are the same version"
    exit 1
fi

current_version="${current_versions[0]}"
next_version=$(echo $setversion_result | sed -r "s/Upgrading \S+ from \S+ to (\S+)/\1\n/" | head -n 1)

# we check if release tag already exists
if git show-ref --tags "v$next_version" --quiet; then
    echo "ERROR: tag already exists for version!"
    exit 1
fi

echo "Making release changes for new release $next_version (last version: $current_version)"

cargo set-version $next_version

sed -i "s/# Unreleased/# Unreleased\n\n# v$next_version/" changelog.md

git checkout -b "release-v$next_version"
git add .
git commit -S -m "chore: bump version to v$next_version"
