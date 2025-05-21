use capnp::capability::Promise;
use capnp_rpc::pry;
use capnp_rpc::{RpcSystem, rpc_twoparty_capnp, twoparty};
use futures::AsyncReadExt;
use serde_json::json;
use zenoh::Config as ZenohConfig;

pub mod schema_capnp {
    include!("app/proto/schema_capnp.rs");
}

use schema_capnp::echo_service;

struct EchoServiceImpl {
    zenoh_session: zenoh::Session,
}

impl echo_service::Server for EchoServiceImpl {
    fn do_echo(
        &mut self,
        params: echo_service::DoEchoParams,
        _results: echo_service::DoEchoResults,
    ) -> capnp::capability::Promise<(), capnp::Error> {
        let data = pry!(pry!(params.get()).get_data());

        let message = pry!(data.get_msg());

        println!("Recv echo msg from client");

        let session = self.zenoh_session.clone();
        let message_string = message.to_string().unwrap();
        println!("Echoing message to zenoh: {}", &message_string);

        tokio::spawn(async move {
            match session.put("do/echo", message_string).await {
                Ok(_) => println!("Echo sent to zenoh on /echo topic"),
                Err(e) => {
                    eprintln!("Failed to publish echo to zenoh: {}", e)
                }
            }
        });

        Promise::ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut config = ZenohConfig::default();
    config
        .insert_json5("mode", &json!("router").to_string())
        .unwrap();
    let session = zenoh::open(config).await.unwrap();

    let addr = "127.0.0.1:7000";

    tokio::task::LocalSet::new()
        .run_until(async move {
            let listener = tokio::net::TcpListener::bind(&addr).await?;
            println!("Listening on {}", &addr);
            let echo_client: schema_capnp::echo_service::Client =
                capnp_rpc::new_client(EchoServiceImpl {
                    zenoh_session: session.clone(),
                });
            println!("Cap n' Proto client created");
            loop {
                let (stream, _) = listener.accept().await?;
                stream.set_nodelay(true)?;
                let (reader, writer) =
                    tokio_util::compat::TokioAsyncReadCompatExt::compat(stream).split();
                let network = twoparty::VatNetwork::new(
                    futures::io::BufReader::new(reader),
                    futures::io::BufWriter::new(writer),
                    rpc_twoparty_capnp::Side::Server,
                    Default::default(),
                );

                let rpc_system =
                    RpcSystem::new(Box::new(network), Some(echo_client.clone().client));

                println!("RPC System created");
                tokio::task::spawn_local(rpc_system);
                println!("RPC System spawned");
            }
        })
        .await
}
