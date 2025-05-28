use roslibrust::{Subscribe, TopicProvider};
use serde_json::json;
use zenoh::try_init_log_from_env;

include!("./messages.rs");

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Initializing roslibrust with Zenoh client...");

    let mut config = zenoh::config::Config::default();
    config
        .insert_json5("mode", &json!("client").to_string())
        .unwrap();
    config
        .insert_json5(
            "connect/endpoints",
            &json!(["tcp/0.0.0.0:7448"]).to_string(),
        )
        .unwrap();

    // Initialize Zenoh session
    let zenoh_session = zenoh::open(zenoh::config::Config::default())
        .await
        .expect("Failed to open Zenoh session");
    println!("Session on ROS: {:?}", zenoh_session);

    try_init_log_from_env();
    // Create roslibrust client with Zenoh backend
    let ros_client = roslibrust::zenoh::ZenohClient::new(zenoh_session);

    // Subscribe to the hello topic using the generated Hello message type
    let mut subscriber = ros_client.subscribe::<test_msg::Hello>("rt/hello").await?;

    println!("✓ Subscribed to 'rt/hello' topic");
    println!("\nWaiting for messages... (send 'goodbye' to exit)");
    println!("{}", "─".repeat(50));

    // Listen for incoming messages
    loop {
        println!("\rWaiting for messages...");
        match subscriber.next().await {
            Ok(payload) => {
                println!("� Received: {:?}", payload);

                // Exit if we receive "goodbye"
                if payload.msg.to_lowercase() == "goodbye" {
                    println!("\n� Received goodbye, shutting down...");
                    break;
                }
            }
            Err(e) => {
                eprintln!("Error receiving message: {}", e);
                break;
            }
        }
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    }

    println!("✓ Shutting down...");
    Ok(())
}
