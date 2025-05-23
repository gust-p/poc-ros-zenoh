use capnp::capability::Promise;
use capnp_rpc::pry;
use capnp_rpc::{RpcSystem, rpc_twoparty_capnp, twoparty};
use futures::AsyncReadExt;
use serde_json::json;
use zenoh::Config as ZenohConfig;

pub mod schema_capnp {
    include!("app/proto/schema_capnp.rs");
}

use schema_capnp::hello_service;
use schema_capnp::twist_service;

struct ZenohService {
    zenoh_session: zenoh::Session,
}

impl hello_service::Server for ZenohService {
    fn do_hello(
        &mut self,
        params: hello_service::DoHelloParams,
        _results: hello_service::DoHelloResults,
    ) -> capnp::capability::Promise<(), capnp::Error> {
        let data = pry!(pry!(params.get()).get_data());

        let message = pry!(data.get_msg());

        println!("Recv hello msg from client");

        let session = self.zenoh_session.clone();
        let message_string = message.to_string().unwrap();
        println!("Helloing message to zenoh: {}", &message_string);

        tokio::spawn(async move {
            match session.put("router/hello", message_string).await {
                Ok(_) => println!("Hello sent to zenoh on /hello topic"),
                Err(e) => {
                    eprintln!("Failed to publish hello to zenoh: {}", e)
                }
            }
        });

        Promise::ok(())
    }
}

impl twist_service::Server for ZenohService {
    fn do_twist(
        &mut self,
        params: twist_service::DoTwistParams,
        _: twist_service::DoTwistResults,
    ) -> capnp::capability::Promise<(), capnp::Error> {
        let data = params.get().unwrap().get_data().unwrap();
        let linear = data.get_linear().unwrap();
        let angular = data.get_angular().unwrap();

        println!(
            "Recv twist msg from client linear: {} {} {} angular: {} {} {}",
            linear.get_x(),
            linear.get_y(),
            linear.get_y(),
            angular.get_x(),
            angular.get_y(),
            angular.get_z()
        );

        Promise::ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut config = ZenohConfig::default();
    config
        .insert_json5("mode", &json!("router").to_string())
        .unwrap();

    println!("Starting with zenoh config: {:?}", &config);
    let session = zenoh::open(config).await.unwrap();

    let addr = "127.0.0.1:7000";

    tokio::task::LocalSet::new()
        .run_until(async move {
            let listener = tokio::net::TcpListener::bind(&addr).await?;
            println!("Listening on {}", &addr);

            let rpc_client: schema_capnp::twist_service::Client =
                capnp_rpc::new_client(ZenohService {
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

                let rpc = RpcSystem::new(Box::new(network), Some(rpc_client.clone().client));

                println!("RPC System created");
                tokio::task::spawn_local(rpc);
                println!("RPC System spawned");
            }
        })
        .await
}
