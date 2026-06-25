use educe::Educe;
use std::net::IpAddr;
use std::net::TcpListener;

/// Transport is for setting which transport mode for the `TestServer`
/// to use when making requests.
#[derive(Educe, Default, Debug)]
#[educe(Clone, PartialEq)]
pub enum Transport {
    /// With this transport mode, `TestRequest` will use a mock HTTP
    /// transport.
    ///
    /// This is the Default Transport type.
    #[default]
    MockHttp,

    /// With this transport mode, a real web server will be spun up
    /// running on a random port. Requests made using the `TestRequest`
    /// will be made over the network stack.
    HttpRandomPort,

    /// With this transport mode, a real web server will be spun up.
    /// Where you can pick which IP and Port to use for this to bind to.
    ///
    /// Setting both `ip` and `port` to `None`, is the equivalent of
    /// using `Transport::HttpRandomPort`.
    HttpIpPort {
        /// Set the IP to use for the server.
        ///
        /// **Defaults** to `127.0.0.1`.
        ip: Option<IpAddr>,

        /// Set the port number to use for the server.
        ///
        /// **Defaults** to a _random_ port.
        port: Option<u16>,
    },

    HttpTcpListner {
        #[educe(
            Clone(method(educe_clone_error)),
            PartialEq(method(educe_return_false))
        )]
        tcp_listener: TcpListener,
    },
}

const fn educe_clone_error(_: &TcpListener) -> TcpListener {
    panic!("Transport::clone is not supported when holding a TcpListener")
}

const fn educe_return_false(_: &TcpListener, _: &TcpListener) -> bool {
    false
}
