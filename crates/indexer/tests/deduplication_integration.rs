use stellarroute_indexer::deduplication::{
    DeduplicationConfig, DeduplicationResult, EventDeduplicator, IdempotencyKey, OrderingStrategy,
};

fn create_test_deduplicator(strategy: OrderingStrategy) -> EventDeduplicator {
    let config = DeduplicationConfig {
        ordering_strategy: strategy,
        ..Default::default()
    };
    EventDeduplicator::new(config)
}

#[tokio::test]
async fn test_duplicate_stream_simulation() {
    let deduplicator = create_test_deduplicator(OrderingStrategy::BestEffort);
    let stream_id = "test-stream";

    let events = vec![
        (IdempotencyKey::from_ledger(100, "tx1", 0), 1),
        (IdempotencyKey::from_ledger(100, "tx1", 1), 2),
        (IdempotencyKey::from_ledger(101, "tx2", 0), 3),
        (IdempotencyKey::from_ledger(101, "tx2", 1), 4),
        (IdempotencyKey::from_ledger(102, "tx3", 0), 5),
    ];

    let mut processed_count = 0;
    for (key, seq) in &events {
        let result = deduplicator.check(key).await;
        if matches!(result, DeduplicationResult::New) {
            processed_count += 1;
            deduplicator.mark_processing(key.clone(), *seq).await;
            deduplicator.mark_completed(key, stream_id, *seq).await;
        }
    }
    assert_eq!(processed_count, 5);

    let mut duplicate_count = 0;
    for (key, _) in &events {
        let result = deduplicator.check(key).await;
        if matches!(result, DeduplicationResult::Duplicate) {
            duplicate_count += 1;
        }
    }
    assert_eq!(duplicate_count, 5);

    let stats = deduplicator.get_stats().await;
    assert_eq!(stats.completed, 5);
}

#[tokio::test]
async fn test_reordered_stream_best_effort() {
    let deduplicator = create_test_deduplicator(OrderingStrategy::BestEffort);
    let stream_id = "horizon-main";

    let out_of_order_events = vec![
        (IdempotencyKey::from_ledger(105, "tx5", 0), 5),
        (IdempotencyKey::from_ledger(103, "tx3", 0), 3),
        (IdempotencyKey::from_ledger(101, "tx1", 0), 1),
        (IdempotencyKey::from_ledger(104, "tx4", 0), 4),
        (IdempotencyKey::from_ledger(102, "tx2", 0), 2),
    ];

    let mut processed = Vec::new();

    for (key, seq) in &out_of_order_events {
        let check = deduplicator.check(key).await;
        if matches!(check, DeduplicationResult::New) {
            let seq_result = deduplicator.check_sequence(stream_id, *seq).await;
            match seq_result {
                Ok(true) => {
                    processed.push(*seq);
                    deduplicator.mark_processing(key.clone(), *seq).await;
                    deduplicator.mark_completed(key, stream_id, *seq).await;
                }
                _ => {}
            }
        }
    }

    assert!(!processed.is_empty());
}

#[tokio::test]
async fn test_reordered_stream_strict() {
    let deduplicator = create_test_deduplicator(OrderingStrategy::StrictSequence);
    let stream_id = "horizon-strict";

    let in_order_events = vec![
        (IdempotencyKey::from_ledger(100, "tx1", 0), 1),
        (IdempotencyKey::from_ledger(101, "tx2", 0), 2),
        (IdempotencyKey::from_ledger(102, "tx3", 0), 3),
    ];

    for (key, seq) in &in_order_events {
        let check = deduplicator.check(key).await;
        if matches!(check, DeduplicationResult::New) {
            let result = deduplicator.check_sequence(stream_id, *seq).await;
            assert!(result.is_ok());
            deduplicator.mark_processing(key.clone(), *seq).await;
            deduplicator.mark_completed(key, stream_id, *seq).await;
        }
    }

    let gap_key = IdempotencyKey::from_ledger(105, "tx6", 0);
    let check = deduplicator.check(&gap_key).await;
    if matches!(check, DeduplicationResult::New) {
        let result = deduplicator.check_sequence(stream_id, 6).await;
        assert!(result.is_err() || matches!(result, Ok(false)));
    }
}

#[tokio::test]
async fn test_mixed_duplicate_and_reordered() {
    let deduplicator = create_test_deduplicator(OrderingStrategy::BestEffort);
    let stream_id = "horizon-mixed";

    let event1 = IdempotencyKey::from_ledger(100, "tx1", 0);
    let event2 = IdempotencyKey::from_ledger(101, "tx2", 0);
    let event3 = IdempotencyKey::from_ledger(102, "tx3", 0);

    assert!(matches!(deduplicator.check(&event1).await, DeduplicationResult::New));
    deduplicator.mark_processing(event1.clone(), 1).await;
    deduplicator.mark_completed(&event1, stream_id, 1).await;

    assert!(matches!(deduplicator.check(&event3).await, DeduplicationResult::New));
    deduplicator.mark_processing(event3.clone(), 3).await;
    deduplicator.mark_completed(&event3, stream_id, 3).await;

    assert!(matches!(deduplicator.check(&event1).await, DeduplicationResult::Duplicate));

    assert!(matches!(deduplicator.check(&event2).await, DeduplicationResult::New));
    let result = deduplicator.check_sequence(stream_id, 2).await;
    assert!(result.is_ok());

    assert!(matches!(deduplicator.check(&event3).await, DeduplicationResult::Duplicate));
}

#[tokio::test]
async fn test_state_persistence_across_restart() {
    let event1 = IdempotencyKey::from_ledger(100, "tx1", 0);
    let event2 = IdempotencyKey::from_ledger(101, "tx2", 0);
    let event3 = IdempotencyKey::from_ledger(102, "tx3", 0);
    let stream_id = "stream1";

    let state = {
        let deduplicator = create_test_deduplicator(OrderingStrategy::BestEffort);

        deduplicator.mark_processing(event1.clone(), 1).await;
        deduplicator.mark_completed(&event1, stream_id, 1).await;
        deduplicator.mark_processing(event2.clone(), 2).await;
        deduplicator.mark_completed(&event2, stream_id, 2).await;

        deduplicator.export_state().await
    };

    let restored_deduplicator = create_test_deduplicator(OrderingStrategy::BestEffort);
    restored_deduplicator.import_state(state).await;

    assert!(matches!(restored_deduplicator.check(&event1).await, DeduplicationResult::Duplicate));
    assert!(matches!(restored_deduplicator.check(&event2).await, DeduplicationResult::Duplicate));
    assert!(matches!(restored_deduplicator.check(&event3).await, DeduplicationResult::New));
}

#[tokio::test]
async fn test_high_volume_duplicate_detection() {
    let deduplicator = create_test_deduplicator(OrderingStrategy::Unordered);
    let stream_id = "high-volume";

    let event_count = 1000u32;
    for i in 0..event_count {
        let key = IdempotencyKey::from_ledger(i, &format!("tx{}", i), 0);
        assert!(matches!(deduplicator.check(&key).await, DeduplicationResult::New));
        deduplicator.mark_processing(key.clone(), i as u64).await;
        deduplicator.mark_completed(&key, stream_id, i as u64).await;
    }

    for i in 0..event_count {
        let key = IdempotencyKey::from_ledger(i, &format!("tx{}", i), 0);
        assert!(matches!(deduplicator.check(&key).await, DeduplicationResult::Duplicate));
    }

    let stats = deduplicator.get_stats().await;
    assert_eq!(stats.completed, event_count as usize);
}

#[tokio::test]
async fn test_interleaved_streams() {
    let deduplicator = create_test_deduplicator(OrderingStrategy::BestEffort);

    let stream_a = "horizon-a";
    let stream_b = "horizon-b";

    let events = vec![
        (stream_a, IdempotencyKey::from_stream(stream_a, 1), 1u64),
        (stream_b, IdempotencyKey::from_stream(stream_b, 1), 1u64),
        (stream_a, IdempotencyKey::from_stream(stream_a, 2), 2u64),
        (stream_b, IdempotencyKey::from_stream(stream_b, 2), 2u64),
        (stream_a, IdempotencyKey::from_stream(stream_a, 3), 3u64),
    ];

    for (stream, key, seq) in &events {
        assert!(matches!(deduplicator.check(key).await, DeduplicationResult::New));
        deduplicator.mark_processing(key.clone(), *seq).await;
        deduplicator.mark_completed(key, stream, *seq).await;
    }

    for (_, key, _) in &events {
        assert!(matches!(deduplicator.check(key).await, DeduplicationResult::Duplicate));
    }

    let stats = deduplicator.get_stats().await;
    assert_eq!(stats.completed, 5);
}

#[tokio::test]
async fn test_failed_event_reprocessing() {
    let mut config = DeduplicationConfig::default();
    config.reprocess_failed = true;
    let deduplicator = EventDeduplicator::new(config);

    let key = IdempotencyKey::from_ledger(100, "tx1", 0);

    assert!(matches!(deduplicator.check(&key).await, DeduplicationResult::New));
    deduplicator.mark_processing(key.clone(), 1).await;
    deduplicator.mark_failed(&key).await;

    assert!(matches!(deduplicator.check(&key).await, DeduplicationResult::Reprocessing));
}
