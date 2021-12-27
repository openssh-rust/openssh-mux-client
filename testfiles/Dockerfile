FROM panubo/sshd

# Since this one is most likely unchanged, run this first
# so it can be cached.
RUN apk update && \
    apk add socat && \
    rm -rf /var/cache/apk/* && \
    echo 'AcceptEnv *'>>/etc/ssh/sshd_config

# These two files are also unlikely to change.
COPY id_rsa.pub /etc/authorized_keys/test
COPY entrypoint.sh /
COPY data /
COPY check_output.sh /

# Env can be changed from time to time.
ENV SSH_USERS=test:1000:1000
ENV TCP_FORWARDING=true

ENTRYPOINT ["/entrypoint.sh"]
