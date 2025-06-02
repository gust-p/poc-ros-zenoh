use capnp::capability::Promise;
use capnp_rpc::pry;
use capnp_rpc::{RpcSystem, rpc_twoparty_capnp, twoparty};
use cdr::{CdrLe, Infinite};
use futures::AsyncReadExt;
use serde::{Deserialize, Serialize};
use zenoh::{Config as ZenohConfig, try_init_log_from_env};

pub mod schema_capnp {
    include!("rpc/schema_capnp.rs");
}

use schema_capnp::bootstrap;
use schema_capnp::hello_service;
use schema_capnp::twist_service;

// Separate service structs for different service types
struct HelloZenohService {
    session: zenoh::Session,
    topic: String,
}

struct TwistZenohService {
    session: zenoh::Session,
    topic: String,
}

#[derive(Deserialize, Serialize, PartialEq)]
struct Hello {
    data: String,
}

#[derive(Deserialize, Serialize, PartialEq)]
struct Vector3 {
    x: f64,
    y: f64,
    z: f64,
}

#[derive(Deserialize, Serialize, PartialEq)]
struct Twist {
    linear: Vector3,
    angular: Vector3,
}

impl hello_service::Server for HelloZenohService {
    fn do_hello(
        &mut self,
        params: hello_service::DoHelloParams,
        _results: hello_service::DoHelloResults,
    ) -> capnp::capability::Promise<(), capnp::Error> {
        let data = pry!(pry!(params.get()).get_data());
        dbg!("received: ", &data);
        let message = pry!(data.get_msg());
        let hello = Hello {
            data: message.to_string().unwrap(),
        };
        let encoded = cdr::serialize::<_, _, CdrLe>(&hello, Infinite).unwrap();

        let session = self.session.clone();
        let topic = self.topic.clone();
        println!("Sending message to zenoh");

        tokio::spawn(async move {
            match session.put(&topic, encoded).await {
                Ok(_) => println!("Hello sent to zenoh on {} topic", topic),
                Err(e) => {
                    eprintln!("Failed to publish hello to zenoh: {}", e)
                }
            }
        });

        Promise::ok(())
    }
}

impl twist_service::Server for TwistZenohService {
    fn do_twist(
        &mut self,
        params: twist_service::DoTwistParams,
        _: twist_service::DoTwistResults,
    ) -> capnp::capability::Promise<(), capnp::Error> {
        let data = params.get().unwrap().get_data().unwrap();
        dbg!("received: ", &data);
        let linear = data.get_linear().unwrap();
        let angular = data.get_angular().unwrap();

        // Serialize twist
        let twist_ser = Twist {
            linear: Vector3 {
                x: linear.get_x(),
                y: linear.get_y(),
                z: linear.get_z(),
            },
            angular: Vector3 {
                x: angular.get_x(),
                y: angular.get_y(),
                z: angular.get_z(),
            },
        };
        let encoded = cdr::serialize::<_, _, CdrLe>(&twist_ser, Infinite).unwrap();

        println!(
            "Publishing twist message to zenoh: x {} y {} z {} angular: x {} y {} z {}",
            linear.get_x(),
            linear.get_y(),
            linear.get_z(),
            angular.get_x(),
            angular.get_y(),
            angular.get_z()
        );

        let session = self.session.clone();
        let topic = self.topic.clone();
        tokio::spawn(async move {
            let publisher = session.declare_publisher(&topic).await.unwrap();
            match publisher.put(encoded).await {
                Ok(_) => println!("Twist sent to zenoh on {} topic", topic),
                Err(e) => {
                    eprintln!("Failed to publish twist to zenoh: {}", e)
                }
            }
        });

        Promise::ok(())
    }
}

// Builder for creating the bootstrap service with configured publishers
pub struct BootstrapServiceBuilder {
    zenoh_session: zenoh::Session,
    hello_topic: Option<String>,
    twist_topic: Option<String>,
}

impl BootstrapServiceBuilder {
    pub fn new(zenoh_session: zenoh::Session) -> Self {
        Self {
            zenoh_session,
            hello_topic: None,
            twist_topic: None,
        }
    }

    pub fn with_hello_publisher(mut self, topic: impl Into<String>) -> Self {
        self.hello_topic = Some(topic.into());
        self
    }

    pub fn with_twist_publisher(mut self, topic: impl Into<String>) -> Self {
        self.twist_topic = Some(topic.into());
        self
    }

    pub async fn build(self) -> Result<BootstrapService, Box<dyn std::error::Error>> {
        Ok(BootstrapService {
            zenoh_session: self.zenoh_session,
            hello_topic: self.hello_topic,
            twist_topic: self.twist_topic,
        })
    }
}

// Bootstrap service that provides access to both services
pub struct BootstrapService {
    zenoh_session: zenoh::Session,
    hello_topic: Option<String>,
    twist_topic: Option<String>,
}

impl bootstrap::Server for BootstrapService {
    fn get_hello_service(
        &mut self,
        _params: bootstrap::GetHelloServiceParams,
        mut results: bootstrap::GetHelloServiceResults,
    ) -> capnp::capability::Promise<(), capnp::Error> {
        if let Some(ref topic) = self.hello_topic {
            let hello_service = capnp_rpc::new_client(HelloZenohService {
                session: self.zenoh_session.clone(),
                topic: topic.clone(),
            });
            results.get().set_service(hello_service);
            Promise::ok(())
        } else {
            Promise::err(capnp::Error::failed(
                "Hello publisher not configured".to_string(),
            ))
        }
    }

    fn get_twist_service(
        &mut self,
        _params: bootstrap::GetTwistServiceParams,
        mut results: bootstrap::GetTwistServiceResults,
    ) -> capnp::capability::Promise<(), capnp::Error> {
        if let Some(ref topic) = self.twist_topic {
            let twist_service = capnp_rpc::new_client(TwistZenohService {
                session: self.zenoh_session.clone(),
                topic: topic.clone(),
            });
            results.get().set_service(twist_service);
            Promise::ok(())
        } else {
            Promise::err(capnp::Error::failed(
                "Twist publisher not configured".to_string(),
            ))
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut config = ZenohConfig::default();
    config.insert_json5("mode", r#""router""#).unwrap();

    println!("Starting with zenoh config: {:?}", &config);
    let session = zenoh::open(config).await.unwrap();
    session.declare_publisher("rt/rosout").await.unwrap();
    println!("Session: {:?}", &session);

    try_init_log_from_env();

    let addr = "0.0.0.0:7000";

    tokio::task::LocalSet::new()
        .run_until(async move {
            let listener = tokio::net::TcpListener::bind(&addr).await?;
            println!("Listening on {}", &addr);

            // Create bootstrap service using builder pattern
            let bootstrap_service = BootstrapServiceBuilder::new(session.clone())
                .with_hello_publisher("fleet/hello")
                .with_twist_publisher("turtle1/cmd_vel")
                .build()
                .await?;

            let bootstrap_client: schema_capnp::bootstrap::Client =
                capnp_rpc::new_client(bootstrap_service);

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
