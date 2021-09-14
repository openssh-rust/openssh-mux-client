#!/bin/bash -ex

cd $(dirname $(realpath $0))

docker build -t sshd .
exec docker run \
    --mount type=bind,src=${PWD},dst=/tmp/output/ \
    -d \
    --name ssh_tester \
    -p 2435:22 \
    --rm \
    sshd 
