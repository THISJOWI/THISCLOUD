use std::sync::Arc;
use tokio::sync::Mutex;

#[tokio::test]
async fn test_publish_subscribe() {
    let bus = thiscloudd::core::EventBus::new();
    let received = Arc::new(Mutex::new(Vec::new()));

    let received_clone = received.clone();
    let _handle = bus.subscribe(move |event| {
        let received = received_clone.clone();
        async move {
            received.lock().await.push(event);
        }
    });

    let test_event = thiscloudd::core::Event::VmCreated {
        vm_id: "test-vm-1".to_string(),
    };

    bus.publish(test_event).await;

    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    let events = received.lock().await;
    assert_eq!(events.len(), 1);
    match &events[0] {
        thiscloudd::core::Event::VmCreated { vm_id } => assert_eq!(vm_id, "test-vm-1"),
        other => panic!("unexpected event: {:?}", other),
    }
}

#[tokio::test]
async fn test_multiple_subscribers() {
    let bus = thiscloudd::core::EventBus::new();
    let count1 = Arc::new(Mutex::new(0u32));
    let count2 = Arc::new(Mutex::new(0u32));

    let count1_clone = count1.clone();
    let _handle1 = bus.subscribe(move |_event| {
        let count = count1_clone.clone();
        async move {
            *count.lock().await += 1;
        }
    });

    let count2_clone = count2.clone();
    let _handle2 = bus.subscribe(move |_event| {
        let count = count2_clone.clone();
        async move {
            *count.lock().await += 1;
        }
    });

    bus.publish(thiscloudd::core::Event::Heartbeat).await;

    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    assert_eq!(*count1.lock().await, 1);
    assert_eq!(*count2.lock().await, 1);
}

#[tokio::test]
async fn test_publish_multiple_events() {
    let bus = thiscloudd::core::EventBus::new();
    let count = Arc::new(Mutex::new(0u32));

    let count_clone = count.clone();
    let _handle = bus.subscribe(move |_event| {
        let count = count_clone.clone();
        async move {
            *count.lock().await += 1;
        }
    });

    bus.publish(thiscloudd::core::Event::Heartbeat).await;
    bus.publish(thiscloudd::core::Event::Heartbeat).await;
    bus.publish(thiscloudd::core::Event::Heartbeat).await;

    tokio::time::sleep(std::time::Duration::from_millis(80)).await;

    assert_eq!(*count.lock().await, 3);
}

#[tokio::test]
async fn test_unsubscribe_on_drop() {
    let bus = thiscloudd::core::EventBus::new();
    let count = Arc::new(Mutex::new(0u32));

    let count_clone = count.clone();
    let handle = bus.subscribe(move |_event| {
        let count = count_clone.clone();
        async move {
            *count.lock().await += 1;
        }
    });

    bus.publish(thiscloudd::core::Event::Heartbeat).await;
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    assert_eq!(*count.lock().await, 1);

    // Dropping the handle should unsubscribe
    drop(handle);

    bus.publish(thiscloudd::core::Event::Heartbeat).await;
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    // Count should still be 1 — handler was removed
    assert_eq!(*count.lock().await, 1);
}
