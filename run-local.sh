#!/usr/bin/env bash
# Serves actual-website-do-not-edit/ at http://localhost:8000
# (kills whatever is already using the port first)
PORT=${PORT:-8000}
cd "$(dirname "$0")" || exit 1
. scripts/require.sh
require python3 https://www.python.org/downloads/ "apt_install python3"
require lsof https://github.com/lsof-org/lsof "apt_install lsof"
cd actual-website-do-not-edit || exit 1

port_in_use() {
    python3 -c "import socket; exit(socket.socket().connect_ex(('127.0.0.1', $PORT)) != 0)"
}

if port_in_use; then
    pids=$(lsof -ti tcp:"$PORT" -sTCP:LISTEN)
    if [ -n "$pids" ]; then
        echo "port $PORT in use, killing: $(ps -o comm= -p $pids | tr '\n' ' ')"
        kill $pids
    else
        # not ours (e.g. a system service), so needs sudo
        echo "port $PORT in use by another user, killing with sudo..."
        pids=$(sudo lsof -ti tcp:"$PORT" -sTCP:LISTEN)
        [ -n "$pids" ] && sudo kill $pids
    fi
    for _ in $(seq 30); do port_in_use || break; sleep 0.1; done
fi

echo "http://localhost:$PORT/en-index.html"
python3 -m http.server "$PORT"
