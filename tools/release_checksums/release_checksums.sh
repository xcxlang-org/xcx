#!/bin/sh
# Release checksum generation for XCX (4.4.1) — POSIX-native,
# no installation required (needs only sh, awk, find, sed and one of:
# sha256sum/sha512sum, shasum or openssl — stock on Linux / macOS /
# FreeBSD and inside Git Bash).
#
# Collects release artifacts and writes two coreutils-format checksum
# sheets — `checksums.sha256` and `checksums.sha512` — so verification is
# a stock command with no warnings on every platform:
#
#     sha256sum -c checksums.sha256      (Linux / FreeBSD)
#     shasum -a 256 -c checksums.sha256  (macOS)
#     Get-FileHash <file> -Algorithm SHA256  (Windows; compare per file)
#
# Each sheet is meant to be signed once (minisign) — the signature then
# covers every artifact listed in it.
#
# Usage:
#   sh tools/release_checksums/release_checksums.sh [--out DIR] \
#       [--patch-iss FILE] ARTIFACT [ARTIFACT ...]
#
# --patch-iss fills the installer's embedded SHA-256 defines
# (`ChecksumXcxExe` -> `xcx.exe`, matched case/punctuation-insensitively,
# an `Exe` suffix on the define name is ignored) BEFORE the installer is
# built. Sheets, `checksums.txt` leftovers and `*.minisig` files are
# always skipped, so re-runs are safe. Artifact names must not contain
# spaces (coreutils sheet format).

set -u

OUT="."
PATCH_ISS=""
ARTIFACTS=""
pending_out=0
pending_iss=0
for a in "$@"; do
    if [ "$pending_out" -eq 1 ]; then OUT="$a"; pending_out=0; continue; fi
    if [ "$pending_iss" -eq 1 ]; then PATCH_ISS="$a"; pending_iss=0; continue; fi
    case "$a" in
        --out) pending_out=1 ;;
        --patch-iss) pending_iss=1 ;;
        *) ARTIFACTS="$ARTIFACTS$a
" ;;
    esac
done
ARTIFACTS=$(printf '%s' "$ARTIFACTS")
if [ -z "$ARTIFACTS" ]; then
    echo "usage: $0 [--out DIR] [--patch-iss FILE] ARTIFACT [ARTIFACT ...]" >&2
    exit 1
fi

# plain hex digest of $1 with algorithm $2 (256|512); tries the stock
# coreutils names first, then shasum (macOS), then openssl
hash_file() {
    if command -v "sha$2sum" > /dev/null 2>&1; then
        "sha$2sum" "$1" | awk '{print $1}'
    elif command -v shasum > /dev/null 2>&1; then
        shasum -a "$2" "$1" | awk '{print $1}'
    elif command -v openssl > /dev/null 2>&1; then
        openssl dgst "-sha$2" "$1" | sed 's/.*= *//'
    else
        echo ""
    fi
}

norm() {
    printf '%s' "$1" | tr -d ' ./-' | tr 'A-Z' 'a-z'
}

LIST="${TMPDIR:-/tmp}/.rc_list.$$"
CLEAN="${TMPDIR:-/tmp}/.rc_clean.$$"
: > "$LIST"

for root in $ARTIFACTS; do
    if [ -f "$root" ]; then
        printf '%s\n' "$root" >> "$LIST"
        continue
    fi
    if [ ! -d "$root" ]; then echo "root not found: $root" >&2; continue; fi
    find "$root" -type f 2>/dev/null | sort >> "$LIST"
done

# drop sheets/leftovers and the patch-iss target from the artifact list
while IFS= read -r f; do
    [ -z "$f" ] && continue
    bn=$(basename "$f")
    case "$bn" in
        checksums.sha256|checksums.sha512|checksums.txt|*.minisig) continue ;;
    esac
    [ -n "$PATCH_ISS" ] && [ "$f" = "$PATCH_ISS" ] && continue
    printf '%s\n' "$f" >> "$CLEAN"
done < "$LIST"
rm -f "$LIST"

FILES=$(cat "$CLEAN")
COUNT=$(printf '%s\n' "$FILES" | wc -l)
if [ -z "$FILES" ] || [ "$COUNT" -eq 0 ]; then
    rm -f "$LIST" "$CLEAN"
    echo "no artifacts found" >&2
    exit 1
fi

if [ -n "$PATCH_ISS" ]; then
    if [ ! -f "$PATCH_ISS" ]; then echo "patch-iss target not found: $PATCH_ISS" >&2; exit 1; fi
    echo "patching $PATCH_ISS:"
    DEFINES=$(sed -n 's/^#define \(Checksum[A-Za-z0-9_]*\) *"[0-9A-Fa-f]*".*/\1/p' "$PATCH_ISS" | sort -u)
    for d in $DEFINES; do
        base=${d#Checksum}
        stembase=$(norm "$base")
        short=${stembase%exe}
        hit=""
        for f in $FILES; do
            bn=$(basename "$f")
            stem=${bn%.*}
            nst=$(norm "$stem")
            if [ "$nst" = "$stembase" ] || [ "$nst" = "$short" ]; then hit="$f"; break; fi
        done
        if [ -z "$hit" ]; then
            echo "  $d SKIPPED (no artifact matches '$base')" >&2
            continue
        fi
        hex=$(hash_file "$hit" 256 | tr 'a-f' 'A-F')
        awk -v d="$d" -v hex="$hex" '
            match($0, "^#define " d "[ \t]*") {
                pre = substr($0, 1, RLENGTH)
                rest = substr($0, RLENGTH + 1)
                if (match(rest, /"[0-9A-Fa-f]*"/)) {
                    $0 = pre "\"" hex "\"" substr(rest, RSTART + RLENGTH)
                }
            }
            { print }
        ' "$PATCH_ISS" > "$PATCH_ISS.mig.$$"
        mv "$PATCH_ISS.mig.$$" "$PATCH_ISS"
        echo "  $d <- $(basename "$hit") ($(printf '%s' "$hex" | cut -c1-16)...)"
    done
fi

for algo in 256 512; do
    : > "$OUT/checksums.sha$algo"
    while IFS= read -r f; do
        [ -z "$f" ] && continue
        printf '%s  %s\n' "$(hash_file "$f" $algo)" "$(basename "$f")" >> "$OUT/checksums.sha$algo"
    done < "$CLEAN"
done

echo "wrote 2 sheets ($COUNT files x 2 algorithms)"
while IFS= read -r f; do
    [ -z "$f" ] && continue
    echo "  $(basename "$f")"
done < "$CLEAN"
echo "next: sign every sheet ->"
echo "  minisign -Sm checksums.sha256 -s <secret key>"
echo "  minisign -Sm checksums.sha512 -s <secret key>"
rm -f "$CLEAN"
exit 0
