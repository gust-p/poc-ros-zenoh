use std::io::{Write, stdout};
use std::process::exit;
use std::time::Duration;

use capnp_rpc::{RpcSystem, rpc_twoparty_capnp, twoparty};
use futures::AsyncReadExt;
use termion::event::{Event, Key};
use termion::input::TermRead;
use termion::raw::IntoRawMode;
use tokio::time::sleep;

pub mod schema_capnp {
    include!("rpc/schema_capnp.rs");
}

use schema_capnp::bootstrap;
use schema_capnp::hello_service;
use schema_capnp::twist_service;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = std::env::var("RPC_SERVER_ADDR").unwrap_or_else(|_| {
        println!("RPC_SERVER_ADDR environment variable must be set");
        exit(1);
    });

    // Print initial instructions to normal stdout
    println!("Cap'n Proto Bootstrap Client");
    println!("Controls:");
    println!("  Right arrow: Send twist message");
    println!("  Left arrow:  Send hello message");
    println!("  Up arrow:    Send forward twist");
    println!("  q:           Exit");
    println!("Connecting to {}...", addr);

    let stdin = std::io::stdin();

    tokio::task::LocalSet::new()
        .run_until(async move {
            let stream = tokio::net::TcpStream::connect(&addr).await?;
            stream.set_nodelay(true)?;
            println!("✓ Connected to {}", &addr);

            let (reader, writer) =
                tokio_util::compat::TokioAsyncReadCompatExt::compat(stream).split();
            let rpc_network = Box::new(twoparty::VatNetwork::new(
                futures::io::BufReader::new(reader),
                futures::io::BufWriter::new(writer),
                rpc_twoparty_capnp::Side::Client,
                Default::default(),
            ));

            let mut rpc_system = RpcSystem::new(rpc_network, None);

            // Get the bootstrap service first
            let bootstrap_client: bootstrap::Client =
                rpc_system.bootstrap(rpc_twoparty_capnp::Side::Server);

            tokio::task::spawn_local(rpc_system);
            println!("✓ RPC System started");

            // Get the hello service from bootstrap
            print!("Getting hello service from bootstrap... ");
            stdout().flush().unwrap();
            let hello_request = bootstrap_client.get_hello_service_request();
            let hello_response = hello_request.send().promise.await?;
            let hello: hello_service::Client = hello_response.get()?.get_service()?;
            println!("✓ Hello service ready");

            // Get the twist service from bootstrap
            print!("Getting twist service from bootstrap... ");
            stdout().flush().unwrap();
            let twist_request = bootstrap_client.get_twist_service_request();
            let twist_response = twist_request.send().promise.await?;
            let twist: twist_service::Client = twist_response.get()?.get_service()?;
            println!("✓ Twist service ready");

            // Send initial hello message
            print!("Sending initial hello message... ");
            stdout().flush().unwrap();
            let mut hello_request = hello.do_hello_request();
            hello_request
                .get()
                .init_data()
                .set_msg("hello from bootstrap client".to_string());
            hello_request.send().promise.await?;
            println!("✓ Initial hello sent");

            println!("\nReady! Use arrow keys to send messages, 'q' to quit.");

            // Put stdin in raw mode only for event handling
            let _stdout = std::io::stdout().into_raw_mode().unwrap();

            // Handle keyboard input
            for c in stdin.events() {
                let evt = c.unwrap();
                match evt {
                    Event::Key(Key::Char('q')) => {
                        println!("\r\nExiting...");
                        break;
                    }
                    Event::Key(Key::Right) => {
                        print!("\r\nSending twist message (right)... ");
                        stdout().flush().unwrap();
                        let mut request_twist = twist.do_twist_request();
                        let mut twist_data = request_twist.get().init_data();
                        let mut linear = twist_data.reborrow().init_linear();
                        linear.set_x(1.0);
                        linear.set_y(0.0);
                        linear.set_z(0.0);
                        let mut angular = twist_data.reborrow().init_angular();
                        angular.set_x(0.0);
                        angular.set_y(0.0);
                        angular.set_z(1.0);
                        request_twist.send().promise.await?;
                        println!("✓ Twist message sent");
                        sleep(Duration::from_millis(100)).await;
                    }
                    Event::Key(Key::Left) => {
                        print!("\r\nSending hello message... ");
                        stdout().flush().unwrap();
                        let mut hello_request = hello.do_hello_request();
                        hello_request
                            .get()
                            .init_data()
                            .set_msg("hello from left arrow".to_string());
                        hello_request.send().promise.await?;
                        println!("✓ Hello message sent");
                        sleep(Duration::from_millis(100)).await;
                    }
                    Event::Key(Key::Up) => {
                        print!("\r\nSending forward twist message... ");
                        stdout().flush().unwrap();
                        let mut request_twist = twist.do_twist_request();
                        let mut twist_data = request_twist.get().init_data();
                        let mut linear = twist_data.reborrow().init_linear();
                        linear.set_x(0.0);
                        linear.set_y(1.0);
                        linear.set_z(0.0);
                        let mut angular = twist_data.reborrow().init_angular();
                        angular.set_x(0.0);
                        angular.set_y(0.0);
                        angular.set_z(-1.0);
                        request_twist.send().promise.await?;
                        println!("✓ Forward twist sent");
                        sleep(Duration::from_millis(100)).await;
                    }
                    _ => {
                        continue;
                    }
                }
            }

            Ok(())
        })
        .await
}
