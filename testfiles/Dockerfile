FROM panubo/sshd

COPY id_rsa.pub /etc/authorized_keys/test
COPY entrypoint.sh /
ENV SSH_USERS=test:1000:1000

ENTRYPOINT ["/entrypoint.sh"]
