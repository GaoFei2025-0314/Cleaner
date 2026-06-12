use c_drive_cleaner::v2::operations::OperationRegistry;

#[test]
fn operation_registry_creates_unique_ids_and_cancels_one_operation() {
    let registry = OperationRegistry::default();
    let first = registry.register();
    let second = registry.register();

    assert_ne!(first.operation_id, second.operation_id);
    assert!(!first.cancelled.load(std::sync::atomic::Ordering::Relaxed));

    assert!(registry.cancel(&first.operation_id));
    assert!(first.cancelled.load(std::sync::atomic::Ordering::Relaxed));
    assert!(!second.cancelled.load(std::sync::atomic::Ordering::Relaxed));
}

#[test]
fn operation_registry_returns_false_for_unknown_cancel_request() {
    let registry = OperationRegistry::default();
    assert!(!registry.cancel("missing-operation"));
}
