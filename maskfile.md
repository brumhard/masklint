# Tasks

## lint

> runs cargo clippy

```bash
cargo clippy -- -W clippy::pedantic
cargo run -- run
```

## testcmd (command)

```bash
if  [ "$command" = "dump" ]; then
    rm -rf "test-out"
    command="dump -o test-out"
fi
# shellcheck disable=SC2086 # needs to be this way to support args in command
cargo run -- $command --maskfile test/maskfile.md
```

## init

> initializes the dev env in the repo

```bash
git config --local core.hooksPath .githooks/
```

## audit

> runs audit for dependencies

```bash
info=$(cargo outdated --root-deps-only --format json)
if [ "$(echo "$info" |  jq '.dependencies | length')" -gt 0 ]; then
    echo "dependencies are not up to date:"
    echo "$info" | jq
    exit 1
fi
vulns=$(cargo audit --json)
if [ "$(echo "$vulns" |  jq '.vulnerabilities.count')" -gt 0 ]; then
    echo "vulnerabilities found:"
    echo "$vulns" | jq
    exit 1
fi
```

## build

> builds the images for (possibly filtered) targets

**OPTIONS**

* filter
  * flags: --filter -f
  * type: string
  * desc: filter all targets with the given string, e.g. "linux", "aarch64"

```bash
set -eo pipefail
targets=$(yq -o json -p toml -r '.toolchain.targets[]' rust-toolchain.toml)
# shellcheck disable=SC2154 # filter is set through mask magic
if [ "$filter" != "" ]; then
    targets=$(echo "$targets" | rg "$filter")
fi

out_dir="out"
# shellcheck disable=SC2115 # out_dir is always assigned
rm -rf "$out_dir"/bin
mkdir -p "$out_dir"/bin

build_args=""
if [ "$verbose" ]; then
    build_args="--verbose"
fi

for target in $targets; do
    echo "building for $target"
    # specifying target-dir is a hack for https://github.com/cross-rs/cross/issues/724
    cross build --release --target "$target" --target-dir "$out_dir/$target" $build_args
    arch_os=$(echo "$target" | 
        rg '^(?P<arch>.+?)-\w+-(?P<os>\w+)(-\w*)?$' -r '${os}_${arch}' |
        sed s/aarch64/arm64/g | 
        sed s/x86_64/amd64/g
    )
    cp "$out_dir/$target/$target/release/masklint" "$out_dir/bin/masklint_$arch_os"
done
```

## tag

> creates a new tag

**OPTIONS**

* next_tag
  * flags: --tag -t
  * desc: tag for the next release version
  * type: string

```bash
set -eo pipefail
if [ "$(git status --porcelain)" != "" ]; then
    echo "nope too dirty"
    exit 1
fi
if [ "$next_tag" = "" ]; then
    current_tag=$(git tag |tail -1)
    proposed_tag=$(svu n)
    read -r -p "Enter next tag or accept proposed (current: '$current_tag', proposed: '$proposed_tag'): " next_tag 
    if [ "$next_tag" = "" ]; then
        next_tag="$proposed_tag"
    fi
fi
# check valid version
if ! echo "$next_tag" | rg -q 'v([0-9]|[1-9][0-9]*)\.([0-9]|[1-9][0-9]*)\.([0-9]|[1-9][0-9]*)'; then
    echo "not a valid version"
    exit 1
fi
# set version without leading v
cargo set-version "${next_tag:1}"
git add Cargo.*
git commit --no-verify --message "chore: bump package to $next_tag"
git tag "$next_tag"
git tag latest --force
git push --no-verify
git push --no-verify --tags --force
```

## test-release

> creates a new release snapshot

**OPTIONS**

* build
  * flags: --build -b
  * type: bool
  * desc: toggle to build before releasing

```bash
if [ "$build" ]; then
    $MASK build
fi
goreleaser release --snapshot --skip-validate --clean --skip-sign
```
