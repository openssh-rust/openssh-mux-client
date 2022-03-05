#!/bin/bash

set -euxo pipefail

cd "$(dirname "$(realpath "$0")")"

chmod 400 id_rsa
rm -f known_host ssh_log

if [ "$1" != "term" ]; then
    options="-nT"
else
    options="${*:2}"
fi

sleep 4

for i in 1 2 3 4 5 6 7 8 9 10; do
    echo The $i try...

    sleep $i
    # shellcheck disable=SC2086
    ssh test@localhost \
        -p 2435 \
        -i id_rsa \
        -o StrictHostKeyChecking=no \
        -o CheckHostIP=no \
        -o UserKnownHostsFile=known_host \
        -o ControlMaster=auto \
        -o ControlPath=/tmp/openssh-mux-client-test.socket \
        -o ControlPersist=yes \
        -F none \
        -E ssh_log \
        -o LogLevel=info \
        $options
    
    exit_code=$?
    if [ $exit_code -eq 0 ]; then
        exit
    fi

    sleep $i
done

echo Failed to start ssh
cat -- *log
exit 1
