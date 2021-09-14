#!/bin/bash

cd $(dirname $(realpath $0))

rm -f known_host ssh_log

if [ "$1" != "term" ]; then
    options="-nT"
fi

for each in 1 2 3 4 5; do
    sleep 1
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
done
