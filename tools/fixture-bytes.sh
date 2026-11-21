# Sourced by Bash fixture tests. The caller owns the file and controls all writes.
fixture_flip_last_byte() {
    local file=$1 size offset byte escape
    [[ -f $file && ! -L $file ]] || { printf 'fixture is not a regular file: %s\n' "$file" >&2; return 1; }
    size=$(stat -c %s -- "$file") || return
    (( size > 0 )) || { printf 'fixture has no final byte: %s\n' "$file" >&2; return 1; }
    offset=$((size - 1))
    byte=$(LC_ALL=C od -An -tu1 -N 1 -j "$offset" -- "$file") || return
    [[ $byte == *([[:blank:]])+([0-9]) ]] || {
        printf 'expected one final fixture byte: %s\n' "$file" >&2
        return 1
    }
    printf -v escape '\\0%03o' "$((byte ^ 1))"
    printf '%b' "$escape" | dd of="$file" bs=1 seek="$offset" conv=notrunc status=none
}
