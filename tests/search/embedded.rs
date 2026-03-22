use std::sync::Arc;
use tokio::time::{sleep, Duration};
use bongas_ai::search::EmbeddedSearchManager;
use bongas_ai::config::types::SearchConfig;
use bongas_ai::pipeline::recovery::fetch_embedded_search::service::EmbeddedSearchStage;
use bongas_ai::pipeline::PipelineStage;
use bongas_ai::pipeline::context::service::ExecutionContext;
use tantivy::doc;
use serde_json::json;

#[tokio::test]
async fn test_embedded_search_integration() {
    let temp_dir = tempfile::tempdir().unwrap();
    let config = SearchConfig {
        enabled: true,
        index_path: temp_dir.path().to_str().unwrap().to_string(),
        writer_memory_mb: 20,
        ..Default::default()
    };
    
    let search_manager = Arc::new(EmbeddedSearchManager::new(config).unwrap());
    let schema = search_manager.schema();

    let doc1 = doc!(
        schema.id => 1i64,
        schema.title => "Friday High Energy Mix",
        schema.description => "Best dance music for your weekend.",
        schema.spoken_native => "This is a high-energy cinematic sequence.",
        schema.vision_dna => vec![0u8; 128],
        schema.metadata => json!({"type": "music"}).to_string()
    );
    
    let doc2 = doc!(
        schema.id => 2i64,
        schema.title => "Gospel Local Flow",
        schema.description => "Songs for the soul.",
        schema.spoken_native => "Hio ngoma inabamba sana, maze naipenda.",
        schema.vision_dna => vec![1u8; 128],
        schema.metadata => json!({"type": "gospel"}).to_string()
    );

    search_manager.upsert_document(doc1).await.unwrap();
    search_manager.upsert_document(doc2).await.unwrap();
    search_manager.commit().unwrap();

    let mut context = ExecutionContext::test_context().await
        .with_search_manager(search_manager.clone());

    // A: Keyword Search
    context.experiment_overrides.insert("q".to_string(), json!("Friday"));
    let stage = EmbeddedSearchStage;
    let results = stage.execute(&context, &json!({"limit": 10}), vec![]).await.unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].item_id, 1);

    // B: Deep Content (Sheng)
    context.experiment_overrides.insert("q".to_string(), json!("inabamba"));
    let results = stage.execute(&context, &json!({"limit": 10}), vec![]).await.unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].item_id, 2);
}

/// Phase 5.1: Sovereign Hardening - Concurrency Stress Test
/// 100 parallel searchers against active background indexing.
#[tokio::test]
async fn test_search_concurrency_stress() {
    let temp_dir = tempfile::tempdir().unwrap();
    let config = SearchConfig {
        enabled: true,
        index_path: temp_dir.path().to_str().unwrap().to_string(),
        writer_memory_mb: 20,
        ..Default::default()
    };
    
    let search_manager = Arc::new(EmbeddedSearchManager::new(config).unwrap());
    let schema = search_manager.schema();

    // 1. Seed initial data
    for i in 0..100 {
        let doc = doc!(
            schema.id => i as i64,
            schema.title => format!("Video Content Item {}", i),
            schema.description => "Stress test item",
            schema.spoken_native => "constant audio pattern",
            schema.vision_dna => vec![0u8; 128],
            schema.metadata => "{}".to_string()
        );
        search_manager.upsert_document(doc).await.unwrap();
    }
    search_manager.commit().unwrap();

    // 2. Spawn 100 concurrent search tasks
    let mut handles = vec![];
    for _ in 0..100 {
        let sm = search_manager.clone();
        let handle = tokio::spawn(async move {
            let mut context = ExecutionContext::test_context().await.with_search_manager(sm);
            context.experiment_overrides.insert("q".to_string(), json!("Video"));
            let stage = EmbeddedSearchStage;
            let results = stage.execute(&context, &json!({"limit": 10}), vec![]).await.unwrap();
            assert!(!results.is_empty());
        });
        handles.push(handle);
    }

    // 3. Background indexing while searching
    let sm_indexing = search_manager.clone();
    let indexing_handle = tokio::spawn(async move {
        for i in 100..110 {
            let doc = doc!(
                sm_indexing.schema().id => i as i64,
                sm_indexing.schema().title => format!("New Video {}", i),
                sm_indexing.schema().description => "New item",
                sm_indexing.schema().spoken_native => "new words",
                sm_indexing.schema().vision_dna => vec![0u8; 128],
                sm_indexing.schema().metadata => "{}".to_string()
            );
            sm_indexing.upsert_document(doc).await.unwrap();
            sm_indexing.commit().unwrap();
            sleep(Duration::from_millis(10)).await;
        }
    });

    for h in handles { h.await.unwrap(); }
    indexing_handle.await.unwrap();
}

/// Phase 5.1: Sovereign Hardening - Linguistic Depth (Sheng/Swahili)
#[tokio::test]
async fn test_sheng_linguistic_depth() {
    let temp_dir = tempfile::tempdir().unwrap();
    let config = SearchConfig {
        enabled: true,
        index_path: temp_dir.path().to_str().unwrap().to_string(),
        writer_memory_mb: 20,
        ..Default::default()
    };
    
    let search_manager = Arc::new(EmbeddedSearchManager::new(config).unwrap());
    let schema = search_manager.schema();

    // Index a complex code-switched item
    let doc = doc!(
        schema.id => 42i64,
        schema.title => "Hio risto inabamba sana",
        schema.description => "Mazee hii ndio lifestyle ya mtaa.",
        schema.spoken_native => "Tunasonga mbele bila uoga, ni gospel flow.",
        schema.vision_dna => vec![0u8; 128],
        schema.metadata => "{}".to_string()
    );
    search_manager.upsert_document(doc).await.unwrap();
    search_manager.commit().unwrap();

    let context = ExecutionContext::test_context().await.with_search_manager(search_manager);
    let stage = EmbeddedSearchStage;

    // Test slang match: "risto"
    let mut ctx = context.clone();
    ctx.experiment_overrides.insert("q".to_string(), json!("risto"));
    let res = stage.execute(&ctx, &json!({"limit": 1}), vec![]).await.unwrap();
    assert_eq!(res[0].item_id, 42);

    // Test Swahili match: "tunasonga"
    let mut ctx = context.clone();
    ctx.experiment_overrides.insert("q".to_string(), json!("tunasonga"));
    let res = stage.execute(&ctx, &json!({"limit": 1}), vec![]).await.unwrap();
    assert_eq!(res[0].item_id, 42);
}

/// Phase 5.1: Sovereign Hardening - Relevance Fusion Edge Cases
#[tokio::test]
async fn test_fusion_no_keyword_match_strong_dna() {
    let temp_dir = tempfile::tempdir().unwrap();
    let config = SearchConfig {
        enabled: true,
        index_path: temp_dir.path().to_str().unwrap().to_string(),
        writer_memory_mb: 20,
        ..Default::default()
    };
    
    let search_manager = Arc::new(EmbeddedSearchManager::new(config).unwrap());
    let schema = search_manager.schema();

    let doc = doc!(
        schema.id => 99i64,
        schema.title => "Abstract Concept",
        schema.description => "No matching keywords here.",
        schema.spoken_native => "Silence.",
        schema.vision_dna => vec![1u8; 128],
        schema.metadata => "{}".to_string()
    );
    search_manager.upsert_document(doc).await.unwrap();
    search_manager.commit().unwrap();

    let mut context = ExecutionContext::test_context().await.with_search_manager(search_manager);
    let stage = EmbeddedSearchStage;

    // Case: Query has no match, but DNA personalization is requested
    context.experiment_overrides.insert("q".to_string(), json!("RandomNonExistentWord"));
    context.experiment_overrides.insert("target_dna_id".to_string(), json!(99));
    
    let res = stage.execute(&context, &json!({"limit": 10, "dna_weight": 1.0}), vec![]).await.unwrap();
    assert!(res.is_empty());
}

#[tokio::test]
async fn test_search_index_auto_repair() {
    let temp_dir = tempfile::tempdir().unwrap();
    let config = SearchConfig {
        enabled: true,
        index_path: temp_dir.path().to_str().unwrap().to_string(),
        index_name: "items".to_string(),
        writer_memory_mb: 20,
        ..Default::default()
    };
    
    {
        let _ = EmbeddedSearchManager::new(config.clone()).unwrap();
    }

    for entry in std::fs::read_dir(temp_dir.path()).unwrap() {
        let path = entry.unwrap().path();
        if path.is_file() {
            std::fs::remove_file(path).unwrap();
        }
    }

    let search_manager = EmbeddedSearchManager::new(config);
    assert!(search_manager.is_ok());
}
