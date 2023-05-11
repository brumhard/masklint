# Tasks

## lint

> runs cargo clippy

```bash
cargo clippy -- -W clippy::pedantic
```

## audit

> runs audit for dependencies

```bash
info=$(cargo outdated --root-deps-only --format json)
if [ $(echo "$info" |  jq '.dependencies | length') -gt 0 ]; then
    echo "dependencies are not up to date:"
    echo "$info" | jq
    exit 1
fi
vulns=$(cargo audit --json)
if [ $(echo "$vulns" |  jq '.vulnerabilities.count') -gt 0 ]; then
    echo "vulnerabilities found:"
    echo "$vulns" | jq
    exit 1
fi
```
