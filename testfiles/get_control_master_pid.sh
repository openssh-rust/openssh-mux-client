#!/bin/bash

exec ssh -o ControlPath=/tmp/openssh-mux-client-test.socket -O check blah 2>&1 | \
    grep -o '(pid=.*)' | \
    sed -e 's/(pid=//' -e 's/)//'
