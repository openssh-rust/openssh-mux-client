#!/bin/bash

set -euxo pipefail

ControlPath=/tmp/openssh-mux-client-test.socket

stop_ssh_tester() {
    exit_code=$?
    testfiles/stop.sh
    exit $exit_code
}

start_ssh_tester() {
    testfiles/start_ssh.sh
    
    ControlMasterPID="$(testfiles/get_control_master_pid.sh)"
    export ControlMasterPID
    if [ -z "$ControlMasterPID" ]; then
        echo Failed to start ssh
        cat testfiles/*log
        exit 1
    fi
}

cd "$(dirname "$(realpath "$0")")"

trap stop_ssh_tester 0

testfiles/start.sh

cargo +nightly miri test non_zero_bytes

start_ssh_tester

if [ $# -lt 1 ]; then
    cargo test test_unordered -- --nocapture
    cargo test test_request_stop_listening -- --nocapture

    if [ -e $ControlPath ]; then
        echo request_stop_listening does not work
        exit 1
    fi

    start_ssh_tester
    cargo test test_sync_request_stop_listening -- --nocapture

    if [ -e $ControlPath ]; then
        echo shutdown_mux_master does not work
        exit 1
    fi
else
    cargo test "$@" -- --nocapture
fi
