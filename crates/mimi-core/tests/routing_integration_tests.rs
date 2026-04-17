//! Integration tests for message routing middleware
//! Tests full topic hierarchy, threading, and realistic message flows

use mimi_core::routing::MessageRouter;
use std::sync::{Arc, Mutex};
use std::thread;

#[test]
fn test_full_topic_hierarchy() {
    let router = MessageRouter::new();

    let commands_called = Arc::new(Mutex::new(false));
    let events_called = Arc::new(Mutex::new(false));
    let memory_called = Arc::new(Mutex::new(false));

    let cmd = commands_called.clone();
    router
        .register("mimi/commands/*", move |_, _| {
            *cmd.lock().unwrap() = true;
            Ok(())
        })
        .unwrap();

    let evt = events_called.clone();
    router
        .register("mimi/events/*", move |_, _| {
            *evt.lock().unwrap() = true;
            Ok(())
        })
        .unwrap();

    let mem = memory_called.clone();
    router
        .register("mimi/memory/*", move |_, _| {
            *mem.lock().unwrap() = true;
            Ok(())
        })
        .unwrap();

    router.route("mimi/commands/execute", b"payload1").unwrap();
    assert!(*commands_called.lock().unwrap());

    router
        .route("mimi/events/state_changed", b"payload2")
        .unwrap();
    assert!(*events_called.lock().unwrap());

    router.route("mimi/memory/store", b"payload3").unwrap();
    assert!(*memory_called.lock().unwrap());
}

#[test]
fn test_wildcard_routing_mimi_catchall() {
    let router = MessageRouter::new();
    let catch_all_called = Arc::new(Mutex::new(0));

    let count = catch_all_called.clone();
    router
        .register("mimi/#", move |_, _| {
            *count.lock().unwrap() += 1;
            Ok(())
        })
        .unwrap();

    router.route("mimi/commands/execute", b"test").unwrap();
    router.route("mimi/events/completed", b"test").unwrap();
    router.route("mimi/memory/store", b"test").unwrap();

    assert_eq!(*catch_all_called.lock().unwrap(), 3);
}

#[test]
fn test_thread_safety_concurrent_routing() {
    let router = Arc::new(MessageRouter::new());
    let success_count = Arc::new(Mutex::new(0));

    let count = success_count.clone();
    router
        .register("mimi/test/*", move |_, _| {
            *count.lock().unwrap() += 1;
            Ok(())
        })
        .unwrap();

    let mut handles = vec![];
    for i in 0..10 {
        let router_clone = router.clone();
        let handle = thread::spawn(move || {
            let topic = format!("mimi/test/message_{}", i);
            router_clone.route(&topic, b"test").unwrap();
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }

    assert_eq!(*success_count.lock().unwrap(), 10);
}

#[test]
fn test_thread_safety_concurrent_registration() {
    let router = Arc::new(MessageRouter::new());

    let mut handles = vec![];
    for i in 0..5 {
        let router_clone = router.clone();
        let handle = thread::spawn(move || {
            let pattern = format!("mimi/test_{}", i);
            router_clone.register(&pattern, |_, _| Ok(())).unwrap();
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }

    assert_eq!(router.list_subscriptions().len(), 5);
}

#[test]
fn test_mixed_exact_and_wildcard_routing() {
    let router = MessageRouter::new();
    let exact_called = Arc::new(Mutex::new(false));
    let wildcard_called = Arc::new(Mutex::new(false));

    let exact = exact_called.clone();
    router
        .register("mimi/commands/execute", move |_, _| {
            *exact.lock().unwrap() = true;
            Ok(())
        })
        .unwrap();

    let wild = wildcard_called.clone();
    router
        .register("mimi/commands/*", move |_, _| {
            *wild.lock().unwrap() = true;
            Ok(())
        })
        .unwrap();

    router.route("mimi/commands/execute", b"test").unwrap();

    assert!(*exact_called.lock().unwrap());
    assert!(*wildcard_called.lock().unwrap());
}
