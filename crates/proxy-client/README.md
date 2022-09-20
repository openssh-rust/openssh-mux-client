# openssh-proxy-client

[![Rust](https://github.com/openssh-rust/openssh-mux-client/actions/workflows/rust.yml/badge.svg)](https://github.com/openssh-rust/openssh-mux-client/actions/workflows/rust.yml)

[![crate.io downloads](https://img.shields.io/crates/d/openssh-proxy-client)](https://crates.io/crates/openssh-proxy-client)

[![crate.io version](https://img.shields.io/crates/v/openssh-proxy-client)](https://crates.io/crates/openssh-proxy-client)

[![docs](https://docs.rs/openssh-proxy-client/badge.svg)](https://docs.rs/openssh-proxy-client)

Rust library to communicate with openssh-mux-server using proxy mode.

The crate is built upon [official document on ssh multiplex protocol][protocol doc]
and [SSH Connection Protocol].

It is currently still in early stage.

## Development

To run tests, make sure you have bash, ssh and docker installed on your computer and run:

```
/path/to/repository/run_test.sh
```

[ssh_format]: https://github.com/openssh-rust/ssh_format
[protocol doc]: https://github.com/openssh/openssh-portable/blob/master/PROTOCOL.mux
[SSH Connection Protocol]: https://www.rfc-editor.org/rfc/rfc4254
