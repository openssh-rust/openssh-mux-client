#!/bin/bash -ex

stop_ssh_tester() {
    source testfiles/stop.sh
}

trap stop_ssh_tester 0

cd $(dirname $(realpath $0))

testfiles/start.sh
testfiles/start_sh.sh

cargo test
