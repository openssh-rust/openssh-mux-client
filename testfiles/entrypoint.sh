#!/bin/bash

exec /entry.sh /usr/sbin/sshd -D -e -f/etc/ssh/sshd_config >/tmp/output/sshd_log 2>&1
