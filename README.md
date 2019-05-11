# another-rust-coin
[![](https://tokei.rs/b1/github/EnsicoinDevs/another-rust-coin)](https://github.com/EnsicoinDevs/another-rust-coin)
[![Build Status](https://travis-ci.com/EnsicoinDevs/another-rust-coin.svg?branch=master)](https://travis-ci.com/EnsicoinDevs/another-rust-coin)

Implementation in Rust of the ensicoin

This implementation uses [tokio](https://tokio.rs/) to run the server. It uses [tower-grpc](https://github.com/tower-rs/tower-grpc/) as a RPC server.

The common ensicoin data types are defined in [ensicoin-messages](https://github.com/EnsicoinDevs/ensicoin-message) and serialization/deserialization in [ensicoin-serializer](https://github.com/EnsicoinDevs/ensicoin-serializer).

## Usage

You first need to bootstrap the blockchain using `arc-bootstrap`. You can specify this utility some parameters that will be used by the daemon.

You can then launch `another-rust-coind`, the daemon. You can connect `arc-prompt` to the daemon to drive some actions.

Using `-h` on any utility will give the detailed list of all options.
