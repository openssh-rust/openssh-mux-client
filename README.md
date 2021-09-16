# openssh-mux-client

Rust library to communicate with openssh-mux-server using [ssh_mux_format].

The entire crate is built upon [official document on ssh multiplex protocol][protocol doc].

Currently, I have written a few test cases to make sure the
 - health check
 - session opening
 - remote port forwarding

are working as intended, while features
 - local port forwarding
 - dynamic forwarding
 - graceful shutdown of the ssh multiplex server
 - terminating the ssh multiplex server

are implemented but not tested.

There are also two features that I didn't implement:
 - forward stdio (stdin + stdout) to remote port (not that useful)
 - closure of port forwarding (according to the [document], it is not implemented yet by ssh)

While it is extremely likely there are bugs in my code, I think it is ready for testing.

## Development

To run tests, make sure you have bash, ssh and docker installed on your computer and run:

```
/path/to/repository/run_test.sh
```

[ssh_mux_format]: https://github.com/NobodyXu/ssh_mux_format
[protocol doc]: https://github.com/openssh/openssh-portable/blob/master/PROTOCOL.mux
