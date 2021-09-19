#!/bin/bash -ex

stop_ssh_tester() {
    source testfiles/stop.sh
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

cargo test test_unordered -- --nocapture
cargo test test_request_stop_listening -- --nocapture
