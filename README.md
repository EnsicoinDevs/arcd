# another-rust-coin
[![](https://tokei.rs/b1/github/EnsicoinDevs/another-rust-coin)](https://github.com/EnsicoinDevs/another-rust-coin)
[![Build Status](https://travis-ci.com/EnsicoinDevs/another-rust-coin.svg?branch=master)](https://travis-ci.com/EnsicoinDevs/another-rust-coin)

Implementation in Rust of the ensicoin

This implementation uses [tokio](https://tokio.rs/) to run the server. It uses [tower-grpc](https://github.com/tower-rs/tower-grpc/) as a RPC server.

The common ensicoin data types are defined in [ensicoin-messages](https://github.com/EnsicoinDevs/ensicoin-message) and serialization/deserialization in [ensicoin-serializer](https://github.com/EnsicoinDevs/ensicoin-serializer).

## Usage

This is a daemon launched with `another-rust-coin`, you can see all availaible options with `-h`.

On its own it can't do very much, you can pair this with a cli like [arc-cli](https://github.com/EnsicoinDevs/arc-cli) to manage the daemon while it runs.

