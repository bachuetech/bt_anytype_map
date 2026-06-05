mod tokio_any_type_tests {
    use bt_anytype_map::anytype::AnyTypeMap;
    use tokio::task;

    #[tokio::test]
    async fn test_basic_insert_and_get() {
        let store = AnyTypeMap::new();

        store.insert(123_i32);
        store.insert(String::from("hello"));

        let a: i32 = store.get().unwrap();
        let b: String = store.get().unwrap();

        assert_eq!(a, 123);
        assert_eq!(b.as_str(), "hello");
    }

    #[tokio::test]
    async fn test_multi_thread_reads() {
        let store = AnyTypeMap::new();
        store.insert(999_i32);

        let mut handles = vec![];

        for _ in 0..32 {
            let store = store.clone();
            handles.push(task::spawn(async move {
                for _ in 0..10_000 {
                    let v: i32 = store.get().unwrap();
                    assert_eq!(v, 999);
                }
            }));
        }

        for h in handles {
            h.await.unwrap();
        }
    }

    #[tokio::test]
    async fn test_multi_thread_writes() {
        let store = AnyTypeMap::new();

        let mut handles = vec![];

        for i in 0..16 {
            let store = store.clone();
            handles.push(task::spawn(async move {
                store.insert(i);
            }));
        }

        for h in handles {
            h.await.unwrap();
        }

        // Last writer wins — value must be one of the inserted ones
        let v: i32 = store.get().unwrap();
        assert!(v >= 0 && v < 16);
    }

    #[tokio::test]
    async fn test_heavy_cas_contention() {
        let store = AnyTypeMap::new();
        store.insert(0usize);

        let mut handles = vec![];

        for _ in 0..32 {
            let store = store.clone();
            handles.push(task::spawn(async move {
                for _ in 0..5000 {
                    let current: usize = store.get().unwrap();
                    store.insert(current + 1);
                }
            }));
        }

        for h in handles {
            h.await.unwrap();
        }

        let final_value: usize = store.get().unwrap();
        assert!(final_value > 0);
    }

    #[tokio::test]
    async fn test_store_moka_cache_and_modify_without_reinsert() {
        use moka::future::Cache;

        let store = AnyTypeMap::new();

        let cache: Cache<String, String> = Cache::builder().build();
        cache.insert("a".into(), "1".into()).await;

        store.insert(cache);

        // Retrieve the same cache instance
        let retrieved: Cache<String, String> = store.get().unwrap();

        // Modify it
        retrieved.insert("b".into(), "2".into()).await;

        // Retrieve again — should see the mutation
        let retrieved2: Cache<String, String> = store.get().unwrap();

        assert_eq!(retrieved2.get(&"a".to_string()).await.unwrap(), "1");
        assert_eq!(retrieved2.get(&"b".to_string()).await.unwrap(), "2");
    }

    #[tokio::test]
    async fn test_parallel_cache_mutation() {
        use moka::future::Cache;

        let store = AnyTypeMap::new();
        let cache: Cache<u32, u32> = Cache::builder().build();
        store.insert(cache);

        let mut handles = vec![];

        for i in 0..32 {
            let store = store.clone();
            handles.push(task::spawn(async move {
                let cache: Cache<u32, u32> = store.get().unwrap();
                for j in 0..1000 {
                    cache.insert(i * 1000 + j, j).await;
                }
            }));
        }

        for h in handles {
            h.await.unwrap();
        }

        let cache: Cache<u32, u32> = store.get().unwrap();

        // Spot check
        assert!(cache.get(&0).await.is_some());
        assert!(cache.get(&500).await.is_some());
        assert!(cache.get(&31_999).await.is_some());
    }
}