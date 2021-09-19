# openssh-mux-client

[![Rust](https://github.com/NobodyXu/concurrency_toolkit/actions/workflows/rust.yml/badge.svg)](https://github.com/NobodyXu/concurrency_toolkit/actions/workflows/rust.yml)

[![crate.io downloads](https://img.shields.io/crates/d/openssh-mux-client)](https://crates.io/crates/openssh-mux-client)

[![crate.io version](https://img.shields.io/crates/v/openssh-mux-client)](https://crates.io/crates/openssh-mux-client)

[![docs](https://docs.rs/openssh-mux-client/badge.svg)](https://docs.rs/openssh-mux-client)

Rust library to communicate with openssh-mux-server using [ssh_mux_format].

The entire crate is built upon [official document on ssh multiplex protocol][protocol doc].

Currently, I have written a few test cases to make sure the
 - health check
 - session opening
 - remote port forwarding
 - graceful shutdown of the ssh multiplex server

are working as intended, while features
 - local port forwarding
 - dynamic forwarding

are implemented but not tested.

There are also two features that I didn't implement:
 - forward stdio (stdin + stdout) to remote port (not that useful)
 - closure of port forwarding (according to the [document], it is not implemented yet by ssh)
 - terminating the ssh multiplex server for the ssh implementation is buggy (the server does not reply with the Ok message before it terminates).

While it is extremely likely there are bugs in my code, I think it is ready for testing.

## Development

To run tests, make sure you have bash, ssh and docker installed on your computer and run:

```
/path/to/repository/run_test.sh
```

[ssh_mux_format]: https://github.com/NobodyXu/ssh_mux_format
[protocol doc]: https://github.com/openssh/openssh-portable/blob/master/PROTOCOL.mux
