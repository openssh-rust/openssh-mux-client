#!/bin/bash -ex

stop_ssh_tester() {
    source testfiles/stop.sh
}

cd $(dirname $(realpath $0))

trap stop_ssh_tester 0

testfiles/start.sh
testfiles/start_ssh.sh

export ControlMasterPID=`testfiles/get_control_master_pid.sh`

cargo test test_unordered -- --nocapture
