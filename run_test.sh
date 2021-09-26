#!/bin/bash -ex

stop_ssh_tester() {
    exit_code=$?
    testfiles/stop.sh
    exit $exit_code
}

cd $(dirname $(realpath $0))

trap stop_ssh_tester 0

testfiles/start.sh
testfiles/start_ssh.sh

export ControlMasterPID=`testfiles/get_control_master_pid.sh`
if [ -z "$ControlMasterPID" ]; then
    echo Failed to start ssh
    cat testfiles/*log
    exit 1
fi

if [ $# -lt 1 ]; then
    cargo test test_unordered -- --nocapture
    cargo test test_request_stop_listening -- --nocapture
else
    cargo test $@ -- --nocapture
fi
