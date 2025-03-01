// Copyright 2019 Parity Technologies (UK) Ltd.
//
// Permission is hereby granted, free of charge, to any person obtaining a
// copy of this software and associated documentation files (the "Software"),
// to deal in the Software without restriction, including without limitation
// the rights to use, copy, modify, merge, publish, distribute, sublicense,
// and/or sell copies of the Software, and to permit persons to whom the
// Software is furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in
// all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS
// OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING
// FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
// DEALINGS IN THE SOFTWARE.

use async_io::Async;
use futures::{
    future::{self, Either},
    prelude::*,
};
use libp2p::core::identity;
use libp2p::core::transport::{self, Transport};
use libp2p::core::upgrade::{self, apply_inbound, apply_outbound, Negotiated};
use libp2p::noise::{
    Keypair, NoiseAuthenticated, NoiseConfig, NoiseError, NoiseOutput, RemoteIdentity, X25519Spec,
    X25519,
};
use libp2p::tcp::TcpTransport;
use log::info;
use quickcheck::*;
use std::{convert::TryInto, io, net::TcpStream};

#[allow(dead_code)]
fn core_upgrade_compat() {
    // Tests API compaibility with the libp2p-core upgrade API,
    // i.e. if it compiles, the "test" is considered a success.
    let id_keys = identity::Keypair::generate_ed25519();
    let noise = NoiseAuthenticated::xx(&id_keys).unwrap();
    let _ = TcpTransport::default()
        .upgrade(upgrade::Version::V1)
        .authenticate(noise);
}

#[test]
fn xx_spec() {
    let _ = env_logger::try_init();
    fn prop(mut messages: Vec<Message>) -> bool {
        messages.truncate(5);
        let server_id = identity::Keypair::generate_ed25519();
        let client_id = identity::Keypair::generate_ed25519();

        let server_id_public = server_id.public();
        let client_id_public = client_id.public();

        let server_dh = Keypair::<X25519Spec>::new()
            .into_authentic(&server_id)
            .unwrap();
        let server_transport = TcpTransport::default()
            .and_then(move |output, endpoint| {
                upgrade::apply(
                    output,
                    NoiseConfig::xx(server_dh),
                    endpoint,
                    upgrade::Version::V1,
                )
            })
            .and_then(move |out, _| expect_identity(out, &client_id_public))
            .boxed();

        let client_dh = Keypair::<X25519Spec>::new()
            .into_authentic(&client_id)
            .unwrap();
        let client_transport = TcpTransport::default()
            .and_then(move |output, endpoint| {
                upgrade::apply(
                    output,
                    NoiseConfig::xx(client_dh),
                    endpoint,
                    upgrade::Version::V1,
                )
            })
            .and_then(move |out, _| expect_identity(out, &server_id_public))
            .boxed();

        run(server_transport, client_transport, messages);
        true
    }
    QuickCheck::new()
        .max_tests(30)
        .quickcheck(prop as fn(Vec<Message>) -> bool)
}

#[test]
fn xx() {
    let _ = env_logger::try_init();
    fn prop(mut messages: Vec<Message>) -> bool {
        messages.truncate(5);
        let server_id = identity::Keypair::generate_ed25519();
        let client_id = identity::Keypair::generate_ed25519();

        let server_id_public = server_id.public();
        let client_id_public = client_id.public();

        let server_dh = Keypair::<X25519>::new().into_authentic(&server_id).unwrap();
        let server_transport = TcpTransport::default()
            .and_then(move |output, endpoint| {
                upgrade::apply(
                    output,
                    NoiseConfig::xx(server_dh),
                    endpoint,
                    upgrade::Version::V1,
                )
            })
            .and_then(move |out, _| expect_identity(out, &client_id_public))
            .boxed();

        let client_dh = Keypair::<X25519>::new().into_authentic(&client_id).unwrap();
        let client_transport = TcpTransport::default()
            .and_then(move |output, endpoint| {
                upgrade::apply(
                    output,
                    NoiseConfig::xx(client_dh),
                    endpoint,
                    upgrade::Version::V1,
                )
            })
            .and_then(move |out, _| expect_identity(out, &server_id_public))
            .boxed();

        run(server_transport, client_transport, messages);
        true
    }
    QuickCheck::new()
        .max_tests(30)
        .quickcheck(prop as fn(Vec<Message>) -> bool)
}

#[test]
fn ix() {
    let _ = env_logger::try_init();
    fn prop(mut messages: Vec<Message>) -> bool {
        messages.truncate(5);
        let server_id = identity::Keypair::generate_ed25519();
        let client_id = identity::Keypair::generate_ed25519();

        let server_id_public = server_id.public();
        let client_id_public = client_id.public();

        let server_dh = Keypair::<X25519>::new().into_authentic(&server_id).unwrap();
        let server_transport = TcpTransport::default()
            .and_then(move |output, endpoint| {
                upgrade::apply(
                    output,
                    NoiseConfig::ix(server_dh),
                    endpoint,
                    upgrade::Version::V1,
                )
            })
            .and_then(move |out, _| expect_identity(out, &client_id_public))
            .boxed();

        let client_dh = Keypair::<X25519>::new().into_authentic(&client_id).unwrap();
        let client_transport = TcpTransport::default()
            .and_then(move |output, endpoint| {
                upgrade::apply(
                    output,
                    NoiseConfig::ix(client_dh),
                    endpoint,
                    upgrade::Version::V1,
                )
            })
            .and_then(move |out, _| expect_identity(out, &server_id_public))
            .boxed();

        run(server_transport, client_transport, messages);
        true
    }
    QuickCheck::new()
        .max_tests(30)
        .quickcheck(prop as fn(Vec<Message>) -> bool)
}

#[test]
fn ik_xx() {
    let _ = env_logger::try_init();
    fn prop(mut messages: Vec<Message>) -> bool {
        messages.truncate(5);
        let server_id = identity::Keypair::generate_ed25519();
        let server_id_public = server_id.public();

        let client_id = identity::Keypair::generate_ed25519();
        let client_id_public = client_id.public();

        let server_dh = Keypair::<X25519>::new().into_authentic(&server_id).unwrap();
        let server_dh_public = server_dh.public_dh_key().clone();
        let server_transport = TcpTransport::default()
            .and_then(move |output, endpoint| {
                if endpoint.is_listener() {
                    Either::Left(apply_inbound(output, NoiseConfig::ik_listener(server_dh)))
                } else {
                    Either::Right(apply_outbound(
                        output,
                        NoiseConfig::xx(server_dh),
                        upgrade::Version::V1,
                    ))
                }
            })
            .and_then(move |out, _| expect_identity(out, &client_id_public))
            .boxed();

        let client_dh = Keypair::<X25519>::new().into_authentic(&client_id).unwrap();
        let server_id_public2 = server_id_public.clone();
        let client_transport = TcpTransport::default()
            .and_then(move |output, endpoint| {
                if endpoint.is_dialer() {
                    Either::Left(apply_outbound(
                        output,
                        NoiseConfig::ik_dialer(client_dh, server_id_public, server_dh_public),
                        upgrade::Version::V1,
                    ))
                } else {
                    Either::Right(apply_inbound(output, NoiseConfig::xx(client_dh)))
                }
            })
            .and_then(move |out, _| expect_identity(out, &server_id_public2))
            .boxed();

        run(server_transport, client_transport, messages);
        true
    }
    QuickCheck::new()
        .max_tests(30)
        .quickcheck(prop as fn(Vec<Message>) -> bool)
}

type Output<C> = (RemoteIdentity<C>, NoiseOutput<Negotiated<Async<TcpStream>>>);

fn run<I, C>(
    mut server: transport::Boxed<Output<C>>,
    mut client: transport::Boxed<Output<C>>,
    messages: I,
) where
    I: IntoIterator<Item = Message> + Clone,
{
    futures::executor::block_on(async {
        server
            .listen_on("/ip4/127.0.0.1/tcp/0".parse().unwrap())
            .unwrap();

        let server_address = server
            .next()
            .await
            .expect("some event")
            .into_new_address()
            .expect("listen address");

        let outbound_msgs = messages.clone();
        let client_fut = async {
            let mut client_session = client
                .dial(server_address.clone())
                .unwrap()
                .await
                .map(|(_, session)| session)
                .expect("no error");

            for m in outbound_msgs {
                let n = (m.0.len() as u64).to_be_bytes();
                client_session.write_all(&n[..]).await.expect("len written");
                client_session.write_all(&m.0).await.expect("no error")
            }
            client_session.flush().await.expect("no error");
        };

        let server_fut = async {
            let mut server_session = server
                .next()
                .await
                .expect("some event")
                .into_incoming()
                .expect("listener upgrade")
                .0
                .await
                .map(|(_, session)| session)
                .expect("no error");

            for m in messages {
                let len = {
                    let mut n = [0; 8];
                    match server_session.read_exact(&mut n).await {
                        Ok(()) => u64::from_be_bytes(n),
                        Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => 0,
                        Err(e) => panic!("error reading len: {}", e),
                    }
                };
                info!("server: reading message ({} bytes)", len);
                let mut server_buffer = vec![0; len.try_into().unwrap()];
                server_session
                    .read_exact(&mut server_buffer)
                    .await
                    .expect("no error");
                assert_eq!(server_buffer, m.0)
            }
        };

        futures::future::join(server_fut, client_fut).await;
    })
}

fn expect_identity<C>(
    output: Output<C>,
    pk: &identity::PublicKey,
) -> impl Future<Output = Result<Output<C>, NoiseError>> {
    match output.0 {
        RemoteIdentity::IdentityKey(ref k) if k == pk => future::ok(output),
        _ => panic!("Unexpected remote identity"),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Message(Vec<u8>);

impl Arbitrary for Message {
    fn arbitrary(g: &mut Gen) -> Self {
        let s = g.gen_range(1..128 * 1024);
        let mut v = vec![0; s];
        for b in &mut v {
            *b = u8::arbitrary(g);
        }
        Message(v)
    }
}
