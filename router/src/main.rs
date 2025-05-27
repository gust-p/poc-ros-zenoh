use capnp::capability::Promise;
use capnp_rpc::pry;
use capnp_rpc::{RpcSystem, rpc_twoparty_capnp, twoparty};
use futures::AsyncReadExt;
use serde_json::json;
use zenoh::{Config as ZenohConfig, try_init_log_from_env};

pub mod schema_capnp {
    include!("rpc/schema_capnp.rs");
}

use schema_capnp::bootstrap;
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
            match session.put("rt/hello", message_string).await {
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
            "Recv twist msg from client linear: x {} y {} z {} angular: x {} y {} z {}",
            linear.get_x(),
            linear.get_y(),
            linear.get_z(),
            angular.get_x(),
            angular.get_y(),
            angular.get_z()
        );

        // Clone session before moving into async block
        let session = self.zenoh_session.clone();

        // Serialize twist data as JSON
        let twist_json = json!({
            "linear": {
                "x": linear.get_x(),
                "y": linear.get_y(),
                "z": linear.get_z()
            },
            "angular": {
                "x": angular.get_x(),
                "y": angular.get_y(),
                "z": angular.get_z()
            }
        });

        println!("Publishing twist message to zenoh: {}", &twist_json);

        tokio::spawn(async move {
            let publisher = session.declare_publisher("turtle1/cmd_vel").await.unwrap();
            match publisher.put(twist_json.to_string()).await {
                Ok(_) => println!("Twist sent to zenoh on /rt/turtle1/cmd_vel topic"),
                Err(e) => {
                    eprintln!("Failed to publish twist to zenoh: {}", e)
                }
            }
        });

        Promise::ok(())
    }
}

// Bootstrap service that provides access to both services
struct BootstrapService {
    zenoh_session: zenoh::Session,
}

impl bootstrap::Server for BootstrapService {
    fn get_hello_service(
        &mut self,
        _params: bootstrap::GetHelloServiceParams,
        mut results: bootstrap::GetHelloServiceResults,
    ) -> capnp::capability::Promise<(), capnp::Error> {
        let hello_service = capnp_rpc::new_client(ZenohService {
            zenoh_session: self.zenoh_session.clone(),
        });

        results.get().set_service(hello_service);
        Promise::ok(())
    }

    fn get_twist_service(
        &mut self,
        _params: bootstrap::GetTwistServiceParams,
        mut results: bootstrap::GetTwistServiceResults,
    ) -> capnp::capability::Promise<(), capnp::Error> {
        let twist_service = capnp_rpc::new_client(ZenohService {
            zenoh_session: self.zenoh_session.clone(),
        });

        results.get().set_service(twist_service);
        Promise::ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut config = ZenohConfig::default();
    config
        .insert_json5("mode", &json!("router").to_string())
        .unwrap();
    // config
    //     .insert_json5(
    //         "listen/endpoints",
    //         &json!(["tcp/127.0.0.1:7447"]).to_string(),
    //     )
    //     .unwrap();
    // config
    //     .insert_json5("scouting/multicast/enabled", &json!(true).to_string())
    //     .unwrap();
    // config
    //     .insert_json5(
    //         "scouting/multicast/address",
    //         &json!("224.0.0.224:7446").to_string(),
    //     )
    //     .unwrap();

    println!("Starting with zenoh config: {:?}", &config);
    let session = zenoh::open(config).await.unwrap();
    println!("Session: {:?}", &session);

    try_init_log_from_env();

    let addr = "0.0.0.0:7000";

    tokio::task::LocalSet::new()
        .run_until(async move {
            let listener = tokio::net::TcpListener::bind(&addr).await?;
            println!("Listening on {}", &addr);

            // Create bootstrap service that provides access to both services
            let bootstrap_client: schema_capnp::bootstrap::Client =
                capnp_rpc::new_client(BootstrapService {
                    zenoh_session: session.clone(),
                });

            println!("Cap n' Proto bootstrap client created");
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

                let rpc = RpcSystem::new(Box::new(network), Some(bootstrap_client.clone().client));

                println!("RPC System created");
                tokio::task::spawn_local(rpc);
                println!("RPC System spawned");
            }
        })
        .await
}
