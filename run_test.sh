#!/bin/bash

set -euxo pipefail

project_dir="$(dirname "$(realpath "$0")")"

ControlPath=/tmp/openssh-mux-client-test.socket

stop_ssh_tester() {
    exit_code=$?
    "$project_dir"/testfiles/stop.sh
    exit $exit_code
}

start_ssh_tester() {
    "$project_dir"/testfiles/start_ssh.sh
    
    ControlMasterPID="$("$project_dir"/testfiles/get_control_master_pid.sh)"
    export ControlMasterPID
    if [ -z "$ControlMasterPID" ]; then
        echo Failed to start ssh
        cat "$project_dir"/testfiles/*log
        exit 1
    fi
}

test_mux_client() {
    cd "$project_dir"/crates/mux-client

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
}

trap stop_ssh_tester 0

"$project_dir"/testfiles/start.sh

test_mux_client "$@"

cd "$project_dir"/crates/mux-client
cargo test "$@"
