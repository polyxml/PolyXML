use std::sync::Arc;

use polyxml::schema::ModelSchema;

#[test]
fn registration_and_clearing_are_visible_to_schema_clones() {
    let base = ModelSchema::builder("Base").build();
    let clone = (*base).clone();
    let derived = ModelSchema::builder("Derived")
        .namespace("urn:derived")
        .build();
    assert!(!clone.has_variants());
    assert!(clone.variants().is_empty());
    base.set_variants(vec![Arc::clone(&derived)]);
    assert!(clone.has_variants());
    assert!(Arc::ptr_eq(&clone.variants()[0], &derived));
    assert!(Arc::ptr_eq(
        &clone.find_variant(b"Derived").unwrap(),
        &derived
    ));
    assert!(clone.find_variant_qname("urn:other", b"Derived").is_none());
    assert!(Arc::ptr_eq(
        &clone.find_variant_qname("urn:derived", b"Derived").unwrap(),
        &derived
    ));
    assert!(clone.matches_variant(&derived));
    clone.set_variants(Vec::new());
    assert!(!base.has_variants());
    assert!(base.variants().is_empty());
    assert!(base.find_variant(b"Derived").is_none());
    assert!(!base.matches_variant(&derived));
    clone.set_variants(vec![Arc::clone(&derived)]);
    assert!(base.has_variants());
    assert!(base.matches_variant(&derived));
}

#[test]
fn concurrent_registry_updates_keep_complete_variant_snapshots() {
    let base = ModelSchema::builder("Base").build();
    let derived = ModelSchema::builder("Derived").build();
    std::thread::scope(|scope| {
        scope.spawn(|| {
            for index in 0..4096 {
                base.set_variants(if index % 2 == 0 {
                    Vec::new()
                } else {
                    vec![Arc::clone(&derived)]
                });
            }
            base.set_variants(vec![Arc::clone(&derived)]);
        });
        for _ in 0..4 {
            scope.spawn(|| {
                for _ in 0..4096 {
                    // These calls can observe different registry revisions.
                    // Each populated snapshot must contain the original Arc.
                    let _ = base.has_variants();
                    let variants = base.variants();
                    assert!(variants.len() <= 1);
                    for variant in variants {
                        assert!(Arc::ptr_eq(&variant, &derived));
                    }
                    if let Some(variant) = base.find_variant(b"Derived") {
                        assert!(Arc::ptr_eq(&variant, &derived));
                    }
                }
            });
        }
    });
    assert!(base.has_variants());
    assert!(Arc::ptr_eq(&base.variants()[0], &derived));
    base.set_variants(Vec::new());
    assert!(!base.has_variants());
    assert!(base.variants().is_empty());
}
