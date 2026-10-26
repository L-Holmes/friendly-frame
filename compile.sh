#!/usr/bin/env bash
# Compiles edit-me/ into actual-website-do-not-edit/
# usage: ./compile.sh [-f|--fresh] [-gp|--github-pages]
cd "$(dirname "$0")" || exit 1
. scripts/require.sh

args=()
for arg in "$@"; do
    case "$arg" in
        -f|--fresh)         args+=(--fresh) ;;
        -gp|--github-pages) args+=(--github-pages) ;;
        *) echo "unknown option: $arg"; echo "usage: ./compile.sh [-f|--fresh] [-gp|--github-pages]"; exit 1 ;;
    esac
done

[ -f "$HOME/.cargo/env" ] && . "$HOME/.cargo/env"
require cargo https://rustup.rs install_rust
require sass https://sass-lang.com/install/ "npm_install_global sass"
if find edit-me -name '*.ts' | grep -q .; then
    require tsc https://www.typescriptlang.org/download/ "npm_install_global typescript"
fi

cargo run -- "${args[@]}"
