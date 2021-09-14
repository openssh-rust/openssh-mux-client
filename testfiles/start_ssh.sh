#!/bin/bash -ex

cd $(dirname $(realpath $0))

rm known_host

if [ "$1" != "term" ]; then
    options="-nT"
fi

exec ssh test@localhost \
    -p 2435 \
    -i id_rsa \
    -o StrictHostKeyChecking=no \
    -o CheckHostIP=no \
    -o UserKnownHostsFile=known_host \
    -o ControlMaster=auto \
    -o ControlPath=/tmp/openssh-mux-client-test.socket \
    -o ControlPersist=yes \
    -F none \
    $options
