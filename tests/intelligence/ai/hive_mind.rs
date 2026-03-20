use tokio::sync::broadcast;
use serde_json::json;
use bongas_ai::engine::intelligence::ai::hive_mind::service::{HiveMindConnector, AdminQuery};
use bongas_ai::config::types::hive_mind::models::HiveMindConfig;

#[tokio::test]
async fn test_admin_query_forensic_skepticism() {
    let (tx, _) = broadcast::channel(1);
    let connector = HiveMindConnector::new(
        HiveMindConfig::default(),
        None,
        tx.subscribe()
    );

    // Case 1: Forensic Mismatch (17+ marked as GE)
    let query = AdminQuery {
        text: "ni GE basi".to_string(),
        context: json!({"forensic_flag": "17+"}),
    };
    let response = connector.process_admin_query(query).await.unwrap();
    assert!(response.text.contains("uko sure wewe?"));
    assert_eq!(response.action.unwrap().get("requires_confirmation").unwrap(), &json!(true));

    // Case 2: Confirmation of manual override
    let query = AdminQuery {
        text: "yes ndio".to_string(),
        context: json!({"pending_override": true}),
    };
    let response = connector.process_admin_query(query).await.unwrap();
    assert!(response.text.contains("ni sawa basi"));
    assert_eq!(response.action.unwrap().get("operation").unwrap(), &json!("manual_override"));
}

#[tokio::test]
async fn test_admin_query_latency_masking() {
    let (tx, _) = broadcast::channel(1);
    let connector = HiveMindConnector::new(
        HiveMindConfig::default(),
        None,
        tx.subscribe()
    );

    let query = AdminQuery {
        text: "confirm this video".to_string(),
        context: json!({}),
    };
    let response = connector.process_admin_query(query).await.unwrap();
    assert!(response.text.contains("kiasi tu"));
    assert_eq!(response.action.unwrap().get("status").unwrap(), &json!("processing"));
}

#[tokio::test]
async fn test_admin_query_context_shielding() {
    let (tx, _) = broadcast::channel(1);
    let connector = HiveMindConnector::new(
        HiveMindConfig::default(),
        None,
        tx.subscribe()
    );

    let query = AdminQuery {
        text: "how is the weather in Nairobi?".to_string(),
        context: json!({}),
    };
    let response = connector.process_admin_query(query).await.unwrap();
    assert!(response.text.contains("wacha tu focus"));
}

#[tokio::test]
async fn test_instant_forensic_interpretation() {
    let (tx, _) = broadcast::channel(1);
    let connector = HiveMindConnector::new(
        HiveMindConfig::default(),
        None,
        tx.subscribe()
    );

    let query = AdminQuery {
        text: "this video is rated as?".to_string(),
        context: json!({}),
    };
    let response = connector.process_admin_query(query).await.unwrap();
    assert_eq!(response.text, "18+");
}

#[tokio::test]
async fn test_admin_query_forbidden_actions() {
    let (tx, _) = broadcast::channel(1);
    let connector = HiveMindConnector::new(
        HiveMindConfig::default(),
        None,
        tx.subscribe()
    );

    let query = AdminQuery {
        text: "delete this video immediately".to_string(),
        context: json!({}),
    };
    let response = connector.process_admin_query(query).await.unwrap();
    assert!(response.text.contains("not allowed to permanently delete"));
    assert_eq!(response.action.unwrap().get("suggested_action").unwrap(), &json!("soft_delete_or_hide"));
}

#[tokio::test]
async fn test_admin_query_technical_translation() {
    let (tx, _) = broadcast::channel(1);
    let connector = HiveMindConnector::new(
        HiveMindConfig::default(),
        None,
        tx.subscribe()
    );

    let query = AdminQuery {
        text: "can you boost the doubling weight for this item?".to_string(),
        context: json!({}),
    };
    let response = connector.process_admin_query(query).await.unwrap();
    assert!(response.text.contains("increase the availability"));
    assert_eq!(response.action.unwrap().get("math_op").unwrap(), &json!("boost_weight"));
}
