use mimi::{ConnectionConfig, ZenohClient};
use std::time::Duration;

#[tokio::test]
#[ignore]
async fn test_full_pub_sub_flow() {
    let cfg = ConnectionConfig::default();
    let client = ZenohClient::new(cfg)
        .await
        .expect("Failed to create client");

    let publisher = client.publisher("test/topic".to_string(), true);
    let mut subscriber = client
        .subscriber("test/topic".to_string(), 10)
        .await
        .expect("Failed to create subscriber");

    tokio::time::sleep(Duration::from_millis(100)).await;

    let data = b"hello zenoh";
    publisher.publish(data).await.expect("Failed to publish");

    let received = tokio::time::timeout(Duration::from_secs(2), subscriber.recv())
        .await
        .expect("Timeout")
        .expect("No message received");

    assert_eq!(received, data.to_vec());

    client.close().await.expect("Failed to close");
}

#[tokio::test]
#[ignore]
async fn test_request_reply_pattern() {
    let cfg = ConnectionConfig::default();
    let client = ZenohClient::new(cfg)
        .await
        .expect("Failed to create client");

    let topic = "rpc/echo".to_string();

    let replier = client.replier(topic.clone(), |req| {
        Ok(format!("echo: {}", String::from_utf8_lossy(&req)).into_bytes())
    });

    replier.start().await.expect("Failed to start replier");

    tokio::time::sleep(Duration::from_millis(100)).await;

    let requester = client.requester(topic, Duration::from_secs(2));

    let request = b"hello";
    let response = requester.request(request).await.expect("Request failed");

    let response_str = String::from_utf8(response).expect("Invalid UTF-8");
    assert!(response_str.contains("echo"));

    client.close().await.expect("Failed to close");
}

#[tokio::test]
#[ignore]
async fn test_connection_pool_distribution() {
    let cfg = ConnectionConfig {
        connection_pool_size: 2,
        ..Default::default()
    };

    let client = ZenohClient::new(cfg)
        .await
        .expect("Failed to create client");
    let pub1 = client.publisher("test/1".to_string(), true);
    let pub2 = client.publisher("test/2".to_string(), true);

    pub1.publish(b"data1").await.expect("Publish 1 failed");
    pub2.publish(b"data2").await.expect("Publish 2 failed");

    client.close().await.expect("Failed to close");
}

#[tokio::test]
#[ignore]
async fn test_timeout_handling() {
    let cfg = ConnectionConfig::default();
    let client = ZenohClient::new(cfg)
        .await
        .expect("Failed to create client");

    let mut subscriber = client
        .subscriber("test/timeout".to_string(), 1)
        .await
        .expect("Failed to create subscriber");

    let result = tokio::time::timeout(
        Duration::from_millis(100),
        subscriber.recv_timeout(Duration::from_millis(50)),
    )
    .await;

    assert!(result.is_ok());

    client.close().await.expect("Failed to close");
}
