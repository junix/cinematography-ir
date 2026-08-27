#!/bin/sh
set -eu

script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd -P)
project_dir=$(dirname -- "$script_dir")
summary="$project_dir/docs/SUMMARY.md"
output="$project_dir/CINEMATOGRAPHY_GUIDE.md"
temporary=$(mktemp "${TMPDIR:-/tmp}/cinematography-guide.XXXXXX")
trap 'rm -f "$temporary"' EXIT HUP INT TERM

{
    printf '%s\n' '# 电影摄影系统化导论与 Cinematography IR'
    printf '%s\n\n' '> 本文件由 `docs/` 中的 mdBook 章节按目录顺序合并，便于单文件阅读。'
    printf '%s\n\n' '---'
    sed -n 's/^- \[[^]]*\](\([^)]*\.md\))$/\1/p' "$summary" | {
        first=1
        while IFS= read -r chapter; do
            if [ "$first" -eq 0 ]; then
                printf '\n%s\n\n' '---'
            fi
            sed -n 'p' "$project_dir/docs/$chapter"
            first=0
        done
    }
} >"$temporary"

mv "$temporary" "$output"
trap - EXIT HUP INT TERM
