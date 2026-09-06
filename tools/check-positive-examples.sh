#!/bin/sh
set -eu

manifest='examples/positive-entrypoints.txt'
[ -f "$manifest" ] || {
    echo "missing positive example manifest: $manifest" >&2
    exit 1
}

# Keep model-return examples explicit about decoded column names.
# Qualified SQL identifiers such as a.id are not model field names unless aliased.
if grep -RInE '^[[:space:]]*SELECT[[:space:]].*[A-Za-z_][A-Za-z0-9_]*\.[A-Za-z_][A-Za-z0-9_]*([[:space:]]*,|[[:space:]]*$)' examples/canonical-url 2>/dev/null | grep -v ' AS '; then
    echo 'positive example contains a qualified model projection without explicit AS alias' >&2
    exit 1
fi

# Every positive example directory containing RWLang source must have one declared
# entrypoint. Security and negative directories are compiler rejection fixtures.
for dir in examples/*; do
    [ -d "$dir" ] || continue
    case "$dir" in
        examples/negative|examples/security) continue ;;
    esac
    find "$dir" -type f -name '*.rw' -print -quit | grep -q . || continue
    if ! grep -Eq "^${dir}/(app|main)\.rw$" "$manifest"; then
        echo "positive example directory is missing from $manifest: $dir" >&2
        exit 1
    fi
done

while IFS= read -r source; do
    case "$source" in
        ''|'#'*) continue ;;
    esac
    [ -f "$source" ] || {
        echo "positive example entrypoint does not exist: $source" >&2
        exit 1
    }
    printf 'checking positive example %s\n' "$source"
    cargo run --locked -q -p rwlang-cli -- check "$source"
done < "$manifest"
