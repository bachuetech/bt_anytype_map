#[cfg(test)]
mod any_type_tests {
    use std::sync::Arc;
    use std::thread;

use bt_anytype_map::anytype::AnyTypeMap;

    // --- Mock Types for Testing Isolation ---
    #[derive(Debug, Clone, PartialEq)]
    struct DatabaseConfig {
        url: String,
    }

    #[derive(Debug, Clone, PartialEq)]
    struct AuthConfig {
        secret: String,
    }

    #[derive(Debug, Clone, Default)]
    struct ConcurrentCounter {
        value: Arc<std::sync::atomic::AtomicU64>,
    }

    // =========================================================================
    // 1. BASIC CRITICAL PATH TESTS
    // =========================================================================

    #[test]
    fn test_insert_and_get() {
        let state = AnyTypeMap::default();

        let db = DatabaseConfig { url: "localhost:5432".to_string() };
        let auth = AuthConfig { secret: "super_secret".to_string() };

        // Insert distinct types
        state.insert(db.clone());
        state.insert(auth.clone());

        // Assert retrieval works and types maintain full isolation
        assert_eq!(state.get::<DatabaseConfig>(), Some(db));
        assert_eq!(state.get::<AuthConfig>(), Some(auth));
    }

    #[test]
    fn test_missing_type_returns_none() {
        let state = AnyTypeMap::new();
        
        // Querying a type that was never registered should cleanly yield None
        assert!(state.get::<DatabaseConfig>().is_none());
    }

    #[test]
    fn test_overwrite_type() {
        let state = AnyTypeMap::new();

        state.insert(DatabaseConfig { url: "url_one".to_string() });
        assert_eq!(state.get::<DatabaseConfig>().unwrap().url, "url_one");

        // Overwriting the same type should smoothly replace it
        state.insert(DatabaseConfig { url: "url_two".to_string() });
        assert_eq!(state.get::<DatabaseConfig>().unwrap().url, "url_two");
    }

    // =========================================================================
    // 2. ADVANCED MULTI-THREADED CONCURRENCY TESTS
    // =========================================================================

    #[test]
    fn test_concurrent_parallel_readers() {
        let state = Arc::new(AnyTypeMap::new());
        let db = DatabaseConfig { url: "pool.internal".to_string() };
        state.insert(db.clone());

        let mut handles = vec![];

        // Spawn 10 parallel threads to blast concurrent reads simultaneously
        for _ in 0..10 {
            let state_clone = Arc::clone(&state);
            let expected_db = db.clone();
            
            let handle = thread::spawn(move || {
                for _ in 0..1000 {
                    let fetched = state_clone.get::<DatabaseConfig>();
                    assert_eq!(fetched, Some(expected_db.clone()));
                }
            });
            handles.push(handle);
        }

        // Wait for all reading threads to terminate successfully
        for handle in handles {
            handle.join().expect("Reader thread panicked");
        }
    }

    #[test]
    fn test_concurrent_read_write_race_conditions() {
        let state = Arc::new(AnyTypeMap::new());
        let mut handles = vec![];

        // Worker A: Constantly reading State
        let state_reader = Arc::clone(&state);
        let read_handle = thread::spawn(move || {
            for _ in 0..5000 {
                // Should never panic or crash, even while writes happen concurrently
                if let Some(config) = state_reader.get::<DatabaseConfig>() {
                    assert!(config.url.starts_with("postgres://"));
                }
                thread::yield_now(); // Force intense thread interleaving
            }
        });
        handles.push(read_handle);

        // Worker B: Constantly writing/updating State at the exact same time
        let state_writer = Arc::clone(&state);
        let write_handle = thread::spawn(move || {
            for i in 0..1000 {
                state_writer.insert(DatabaseConfig {
                    url: format!("postgres://node-{}", i),
                });
            }
        });
        handles.push(write_handle);

        for handle in handles {
            handle.join().expect("Thread crashed under lock-free race conditions");
        }
    }

    #[test]
    fn test_interior_mutability_across_threads() {
        let state = Arc::new(AnyTypeMap::new());
        let counter = ConcurrentCounter::default();
        state.insert(counter.clone());

        let mut handles = vec![];

        // Spawn 8 threads modifying shared data inside a single plugin instance
        for _ in 0..8 {
            let state_clone = Arc::clone(&state);
            let handle = thread::spawn(move || {
                for _ in 0..500 {
                    // Fetch cloned reference pointer to the plugin
                    if let Some(plugin) = state_clone.get::<ConcurrentCounter>() {
                        // Increment inner atomic counter (No map re-insert needed)
                        plugin.value.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    }
                }
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.join().expect("Worker failed updating shared state data");
        }

        // Assert 8 threads * 500 increments = exactly 4000
        let final_plugin = state.get::<ConcurrentCounter>().unwrap();
        assert_eq!(final_plugin.value.load(std::sync::atomic::Ordering::SeqCst), 4000);
    }
}