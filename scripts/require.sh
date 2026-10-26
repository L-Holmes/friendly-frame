# Makes sure a command is installed. If it isn't:
#   - debian / ubuntu etc. (has apt-get): asks for <enter>, then installs it
#   - anything else: prints where to get it, and exits
# usage: require <command> <website> <install command>
require() {
    command -v "$1" >/dev/null 2>&1 && return 0

    echo "$1 is not installed. ($2)"
    if command -v apt-get >/dev/null 2>&1 && [ -t 0 ]; then
        echo "Press <enter> to install it automatically (or ctrl+c to cancel)"
        read -r
        if eval "$3"; then
            hash -r
            command -v "$1" >/dev/null 2>&1 && return 0
        fi
        echo "Couldn't install $1 automatically, install it manually from: $2"
    fi
    exit 1
}

apt_install() {
    sudo apt-get update -qq
    sudo apt-get install -y "$@"
}

# Installs an npm package globally.
# Only installs npm if it's missing (node from e.g. nodesource already comes with npm, and debian's npm package clashes with it)
npm_install_global() {
    command -v npm >/dev/null 2>&1 || apt_install npm || return 1
    if [ -w "$(npm prefix -g)" ]; then
        npm install -g "$@"     # e.g. nvm, no sudo needed
    else
        sudo env PATH="$PATH" npm install -g "$@"
    fi
}

install_rust() {
    command -v curl >/dev/null 2>&1 || apt_install curl || return 1
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y && . "$HOME/.cargo/env"
}
