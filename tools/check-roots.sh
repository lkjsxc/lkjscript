#!/bin/sh
# Inventory the handwritten driver-root registrations, not general source syntax.
# Native admission must not read the optional witness tree.
# Usage: check-roots.sh [--witness] [TREE]
set -eu
mode=native
if [ "${1-}" = --witness ]; then mode=witness; shift; fi
argument=${1-}
if [ "$#" -gt 1 ] || [ "${argument#-}" != "$argument" ]; then
    echo 'usage: check-roots.sh [--witness] [TREE]' >&2
    exit 2
fi
cd "${1:-$(dirname "$0")/..}"
if [ "$mode" = native ]; then
    set -- src/main.lkj src/programs_admission_main.lkj
else
    set -- src/main.lkj witness/bridge/main.rs witness/build.rs
fi
awk -v mode="$mode" '
# Only declarations, wrapper includes and the producer array are inventoried.
# Tokenize comments/literals once so formatting cannot invent or hide bindings.
function fail(message) { print message > "/dev/stderr"; bad = 1 }
function ident(value) { return value ~ /^[A-Za-z_][A-Za-z_0-9]*$/ }
function emit(value, kind) { token[++count] = value; token_kind[count] = kind }
function string_token() { emit("\"" literal "\"", "string"); string_value[count] = literal; literal = "" }
function lex(text,    i, c, pair, rest, size) {
    for (i = 1; i <= length(text); i++) {
        c = substr(text, i, 1); pair = substr(text, i, 2)
        if (block) {
            if (pair == "/*") { block++; i++ }
            else if (pair == "*/") { block--; i++ }
        } else if (raw != "") {
            if (substr(text, i, length(raw)) == raw) { i += length(raw) - 1; raw = ""; string_token() }
            else literal = literal c
        } else if (quoted) {
            if (escaped) { literal = literal c; escaped = 0 }
            else if (c == "\\") { literal = literal c; escaped = 1 }
            else if (c == "\"") { quoted = 0; string_token() }
            else literal = literal c
        } else if (pair == "//") break
        else if (pair == "/*") { block = 1; i++ }
        else if (c !~ /[ \t\r]/) {
            rest = substr(text, i)
            if (match(rest, /^(b)?r#*"/)) {
                size = RLENGTH; rest = substr(rest, 1, size); sub(/^b/, "", rest)
                raw = "\"" substr(rest, 2, length(rest) - 2); literal = ""; i += size - 1
            } else if (c == "\"") { quoted = 1; literal = "" }
            else if (c == apostrophe && match(rest, "^" apostrophe "([^" apostrophe "\\\\]|\\\\.)" apostrophe)) {
                emit(substr(rest, 1, RLENGTH), "char"); i += RLENGTH - 1
            } else if (match(rest, /^[A-Za-z_][A-Za-z_0-9]*/)) {
                emit(substr(rest, 1, RLENGTH), "identifier"); i += RLENGTH - 1
            } else if (pair == "::") { emit(pair, "punctuation"); i++ }
            else emit(c, "punctuation")
        }
    }
    # A physical newline consumes a preceding escape in a continued string.
    escaped = 0
    if (quoted || raw != "") literal = literal "\n"
}
function closing(start, open, closer, limit,    i, depth) {
    for (i = start; i <= limit; i++) {
        if (token[i] == open) depth++
        else if (token[i] == closer && --depth == 0) return i
    }
    fail(current ": unterminated " open " in root inventory"); return 0
}
function register(role, name) {
    if (name == "_") return
    if (role == "main") main[name] = 1
    else if (role == "admission") admission[name] = 1
    else bridge[name] = 1
}
function imports(start, end, role,    namespace, p, last, source, binding) {
    namespace = role == "bridge" ? "lkjscript" : "std"
    if (token[start] != namespace || token[start + 1] != "::") return
    p = start + 2; last = end - 1
    if (token[p] == "{") {
        if (token[last] != "}") { fail(current ": malformed root import"); return }
        p++; last--
    }
    while (p <= last) {
        source = token[p++]; binding = source
        if (!ident(source)) { fail(current ": unsupported root import"); return }
        if (token[p] == "as") { binding = token[p + 1]; p += 2 }
        if (!ident(binding) || (p <= last && token[p] != ",")) {
            fail(current ": unsupported root import for " source); return
        }
        if (role == "bridge" || source == "sha256") register(role, binding)
        if (p <= last) p++
    }
}
function roots(role,    i, j, end, p, name, attributed) {
    i = 1
    while (i <= count) {
        if (token[i] == "#" && token[i + 1] == "!" && token[i + 2] == "[") {
            end = closing(i + 2, "[", "]", count); if (!end) return
            i = end + 1; continue
        }
        if (token[i] == "#" && token[i + 1] == "[") {
            # Native conditional/path registrations need a deliberate inventory rule.
            end = closing(i + 1, "[", "]", count); if (!end) return
            attributed = 1; i = end + 1; continue
        }
        j = i
        if (token[j] == "pub") {
            j++
            if (token[j] == "(") {
                end = closing(j, "(", ")", count); if (!end) return
                j = end + 1
            }
        }
        if (token[j] == "mod" && ident(token[j + 1]) && (token[j + 2] == ";" || token[j + 2] == "{")) {
            if (role == "bridge" || !attributed) register(role, token[j + 1])
            else fail(current ": attributed driver-root module " token[j + 1] " needs an inventory rule")
            i = j + 3
            if (token[j + 2] == "{") {
                end = closing(j + 2, "{", "}", count); if (!end) return
                i = end + 1
            }
        } else if (token[j] == "use") {
            end = j + 1
            while (end <= count && token[end] != ";") end++
            if (end > count) { fail(current ": unterminated root import"); return }
            if (role == "bridge" || !attributed) imports(j + 1, end, role)
            else fail(current ": attributed root import needs an inventory rule")
            i = end + 1
        } else if (token[j] == "include" && token[j + 1] == "!" && token[j + 2] == "(") {
            end = closing(j + 2, "(", ")", count); if (!end) return
            if (role == "bridge") {
                for (p = j + 3; p < end; p++) {
                    name = string_value[p]
                    if (token_kind[p] == "string" && name ~ /^\/[A-Za-z_][A-Za-z_0-9]*-module\.rs$/) {
                        name = substr(name, 2, length(name) - 11)
                        bridge[name] = 1; included[name] = 1
                    }
                }
            }
            i = end + 1
            if (token[i] == ";") i++
        } else {
            # Skip other items and their bodies; nested modules never count as roots.
            while (i <= count) {
                if (token[i] == "(" || token[i] == "[" || token[i] == "{") {
                    name = token[i]; end = closing(i, name, name == "(" ? ")" : name == "[" ? "]" : "}", count)
                    if (!end) return
                    i = end + 1
                    if (name == "{") break
                } else if (token[i++] == ";") break
            }
        }
        attributed = 0
    }
}
function producers(    i, j, start, end, table_end, tables, entries, file, name) {
    # Only the wrapper-producing loop in main, not similarly shaped helper data.
    for (i = 1; i <= count; i++) {
        if (token[i] == "fn" && token[i + 1] == "main") {
            start = i + 2
            while (start <= count && token[start] != "{") start++
            end = closing(start, "{", "}", count); break
        }
        if (token[i] == "{") { i = closing(i, "{", "}", count); if (!i) return }
    }
    if (end) for (i = start + 1; i < end; i++) {
        if (token[i] != "for" || token[i + 1] != "(" || token[i + 2] != "file" ||
            token[i + 3] != "," || token[i + 4] != "module") continue
        j = i + 5
        if (token[j] == ",") j++
        if (token[j] != ")" || token[j + 1] != "in" || token[j + 2] != "[") continue
        tables++; j += 2
        table_end = closing(j, "[", "]", end); if (!table_end) return
        j++
        while (j < table_end) {
            if (token[j] != "(" || token_kind[j + 1] != "string" ||
                token[j + 2] != "," || token_kind[j + 3] != "string") {
                fail(current ": unsupported wrapper producer tuple"); return
            }
            file = string_value[j + 1]; name = string_value[j + 3]; j += 4
            if (token[j] == ",") j++
            if (token[j++] != ")" || (j < table_end && token[j] != ",")) {
                fail(current ": malformed wrapper producer tuple"); return
            }
            entries++
            # The language wrapper is consumed by witness/bridge/lib.rs.
            if (!(file == "lib.rs" && name == "language")) {
                if (!ident(name) || file != name ".rs") fail(current ": wrapper " name " has mismatched source " file)
                produced[name] = 1
            }
            if (j < table_end) j++
        }
        i = table_end
    }
    producer_parsed = tables == 1 && entries > 0
}
function finish_file() {
    if (block || quoted || raw != "") fail(current ": unterminated comment or literal")
    if (current == "src/main.lkj") roots("main")
    else if (current == "src/programs_admission_main.lkj") roots("admission")
    else if (current == "witness/bridge/main.rs") roots("bridge")
    else if (current == "witness/build.rs") producers()
}
BEGIN { apostrophe = sprintf("%c", 39) }
FNR == 1 {
    if (current != "") finish_file()
    for (key in token) delete token[key]
    for (key in token_kind) delete token_kind[key]
    for (key in string_value) delete string_value[key]
    count = block = quoted = escaped = 0; raw = literal = ""; current = FILENAME
}
{ lex($0) }
END {
    if (current != "") finish_file()
    if (!("driver" in main)) fail("src/main.lkj: no driver module parsed; refusing a vacuous check")
    if (mode == "native") {
        for (m in main) if (!(m in admission)) fail("src/programs_admission_main.lkj: missing driver-root module " m)
        for (m in admission) if (!(m in main)) fail("src/programs_admission_main.lkj: module " m " is absent from src/main.lkj")
    } else {
        if (!producer_parsed) fail("witness/build.rs: no single nonempty producer table parsed; refusing a vacuous check")
        # facts is library-private. Extra bridge helpers are intentional; Cargo
        # checks their definitions, while generated wrapper pairing is symmetric.
        for (m in main) if (m != "facts" && !(m in bridge)) fail("witness/bridge/main.rs: missing driver-root module " m)
        for (m in included) if (!(m in produced)) fail("witness/build.rs: no producer for included " m "-module.rs")
        for (m in produced) if (!(m in included)) fail("witness/bridge/main.rs: produced " m "-module.rs is not included")
    }
    if (bad) exit 1
}' "$@"
printf 'ok %s driver roots\n' "$mode"
