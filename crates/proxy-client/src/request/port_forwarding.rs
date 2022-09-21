use serde::Serialize;

use super::Request;
use crate::{constants::*, IpAddr};

#[derive(Copy, Clone, Debug, Serialize)]
pub(crate) struct GlobalRequest<T> {
    request_name: &'static &'static str,
    want_reply: bool,
    request_data: T,
}

impl<T: Serialize> GlobalRequest<T> {
    fn new(request_name: &'static &'static str, request_data: T) -> Request<Self> {
        Request::new(
            SSH_MSG_GLOBAL_REQUEST,
            Self {
                request_name,
                want_reply: true,
                request_data,
            },
        )
    }
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct RemoteForward<'a>(IpAddr<'a>);

impl<'a> RemoteForward<'a> {
    /// The 'address to bind' and 'port number to bind' specify the IP
    /// address (or domain name) and port on which connections for forwarding
    /// are to be accepted.  Some strings used for 'address to bind' have
    /// special-case semantics.
    ///
    ///  - "" means that connections are to be accepted on all protocol
    ///    families supported by the SSH implementation.
    ///  - "0.0.0.0" means to listen on all IPv4 addresses.
    ///  - "::" means to listen on all IPv6 addresses.
    ///  - "localhost" means to listen on all protocol families supported by
    ///    the SSH implementation on loopback addresses only ([RFC3330] and
    ///    [RFC3513]).
    ///  - "127.0.0.1" and "::1" indicate listening on the loopback
    ///    interfaces for IPv4 and IPv6, respectively.
    ///
    /// Note that the client can still filter connections based on
    /// information passed in the open request.
    ///
    /// Implementations should only allow forwarding privileged ports if the
    /// user has been authenticated as a privileged user.
    ///
    /// Client implementations SHOULD reject these messages; they are
    /// normally only sent by the client.
    ///
    /// If a client passes 0 as port number to bind and has 'want reply' as
    /// TRUE, then the server allocates the next available unprivileged port
    /// number and replies with the following message; otherwise, there is no
    /// response-specific data.
    pub(crate) fn new(ip_addr: IpAddr<'a>) -> Request<GlobalRequest<Self>> {
        GlobalRequest::new(&"tcpip-forward", Self(ip_addr))
    }
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct CancelRemoteForward<'a>(IpAddr<'a>);

impl<'a> CancelRemoteForward<'a> {
    pub(crate) fn new(ip_addr: IpAddr<'a>) -> Request<GlobalRequest<Self>> {
        GlobalRequest::new(&"cancel-tcpip-forward", Self(ip_addr))
    }
}
