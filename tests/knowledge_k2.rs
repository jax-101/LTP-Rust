use std::process::Command;

use serde_json::Value;

fn ltp_bin() -> String {
    env!("CARGO_BIN_EXE_ltp").to_string()
}

fn run_ltp(dir: &std::path::Path, args: &[&str]) -> (Value, i32) {
    let output = Command::new(ltp_bin())
        .args(args)
        .current_dir(dir)
        .output()
        .expect("failed to execute ltp binary");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let code = output.status.code().unwrap_or(-1);

    let json: Value = serde_json::from_str(&stdout).unwrap_or_else(|_| {
        panic!(
            "Failed to parse JSON output.\nstdout: {}\nstderr: {}",
            stdout,
            String::from_utf8_lossy(&output.stderr)
        )
    });

    (json, code)
}

fn init_workspace(dir: &std::path::Path) {
    let (json, code) = run_ltp(dir, &["init", "--name", "TestKnowledge"]);
    assert_eq!(code, 0, "init failed: {:?}", json);
}

// === K2.1: Add with all fields ===
#[test]
fn k2_1_add_measurement_all_fields() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    init_workspace(dir);

    let (json, code) = run_ltp(
        dir,
        &[
            "knowledge",
            "add",
            "Media 18.3 dias",
            "--type",
            "measurement",
            "--source-excerpt",
            "ERP Q2",
            "--status",
            "verified",
            "--confidence",
            "high",
        ],
    );

    assert_eq!(code, 0);
    assert_eq!(json["success"], true);
    assert_eq!(json["action"], "knowledge_add");
    assert_eq!(json["data"]["created_knowledge_id"], "KN-001");
    assert_eq!(json["data"]["type"], "measurement");
    assert_eq!(json["data"]["label"], "Media 18.3 dias");
    assert_eq!(json["data"]["status"], "verified");
    assert_eq!(json["data"]["confidence"], "high");

    // Verify on disk
    let kn_path = dir.join("knowledge/KN-001.json");
    assert!(kn_path.exists());
    let content: Value = serde_json::from_str(&std::fs::read_to_string(kn_path).unwrap()).unwrap();
    assert_eq!(content["id"], "KN-001");
    assert_eq!(content["type"], "measurement");
    assert_eq!(content["status"], "verified");
}

// === K2.2: Add with defaults (status=unverified, confidence=medium) ===
#[test]
fn k2_2_add_testimony_defaults() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    init_workspace(dir);

    let (json, code) = run_ltp(
        dir,
        &[
            "knowledge",
            "add",
            "Director dice mejoraron",
            "--type",
            "testimony",
            "--source-uri",
            "slack://C04/p123",
        ],
    );

    assert_eq!(code, 0);
    assert_eq!(json["data"]["created_knowledge_id"], "KN-001");
    assert_eq!(json["data"]["status"], "unverified");
    assert_eq!(json["data"]["confidence"], "medium");
}

// === K2.3: Add with tags ===
#[test]
fn k2_3_add_with_tags() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    init_workspace(dir);

    let (_json, code) = run_ltp(
        dir,
        &[
            "knowledge",
            "add",
            "Hipotesis",
            "--type",
            "hypothesis",
            "--source-excerpt",
            "Entrevista",
            "--tags",
            "logistica,proveedor",
        ],
    );

    assert_eq!(code, 0);
    let kn_path = dir.join("knowledge/KN-001.json");
    let content: Value = serde_json::from_str(&std::fs::read_to_string(kn_path).unwrap()).unwrap();
    let tags = content["tags"].as_array().unwrap();
    assert!(tags.contains(&Value::String("logistica".to_string())));
    assert!(tags.contains(&Value::String("proveedor".to_string())));
}

// === K2.4: Add without source fails with SOURCE_REQUIRED ===
#[test]
fn k2_4_add_no_source_fails() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    init_workspace(dir);

    let (json, code) = run_ltp(
        dir,
        &["knowledge", "add", "Sin fuente", "--type", "observation"],
    );

    assert_ne!(code, 0);
    assert_eq!(json["success"], false);
    assert_eq!(json["errors"][0]["code"], "SOURCE_REQUIRED");
    // Counter NOT incremented
    assert!(!dir.join("knowledge/KN-001.json").exists());
}

// === K2.5: Add with empty label fails ===
#[test]
fn k2_5_add_empty_label_fails() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    init_workspace(dir);

    let (json, code) = run_ltp(
        dir,
        &[
            "knowledge",
            "add",
            "",
            "--type",
            "measurement",
            "--source-excerpt",
            "x",
        ],
    );

    assert_ne!(code, 0);
    assert_eq!(json["success"], false);
    assert_eq!(json["errors"][0]["code"], "LABEL_REQUIRED");
}

// === K2.6: Add with invalid type fails ===
#[test]
fn k2_6_add_invalid_type_fails() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    init_workspace(dir);

    let (json, code) = run_ltp(
        dir,
        &[
            "knowledge",
            "add",
            "Test",
            "--type",
            "INVALID",
            "--source-excerpt",
            "x",
        ],
    );

    assert_ne!(code, 0);
    assert_eq!(json["success"], false);
    assert_eq!(json["errors"][0]["code"], "INVALID_KNOWLEDGE_TYPE");
}

// === K2.7: Add with both source fields empty ===
#[test]
fn k2_7_add_both_sources_empty() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    init_workspace(dir);

    let (json, code) = run_ltp(
        dir,
        &[
            "knowledge",
            "add",
            "Test",
            "--type",
            "measurement",
            "--source-uri",
            "",
            "--source-excerpt",
            "",
        ],
    );

    assert_ne!(code, 0);
    assert_eq!(json["errors"][0]["code"], "SOURCE_REQUIRED");
}

// === K2.8: Add with invalid confidence ===
#[test]
fn k2_8_add_invalid_confidence() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    init_workspace(dir);

    let (json, code) = run_ltp(
        dir,
        &[
            "knowledge",
            "add",
            "Test",
            "--type",
            "measurement",
            "--source-uri",
            "valid",
            "--confidence",
            "invalid_value",
        ],
    );

    assert_ne!(code, 0);
    assert_eq!(json["errors"][0]["code"], "INVALID_CONFIDENCE");
}

// === K2.9: Large label and excerpt accepted ===
#[test]
fn k2_9_large_fields_accepted() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    init_workspace(dir);

    let big_label = "A".repeat(5000);
    let big_excerpt = "B".repeat(10000);

    let (_json, code) = run_ltp(
        dir,
        &[
            "knowledge",
            "add",
            &big_label,
            "--type",
            "measurement",
            "--source-excerpt",
            &big_excerpt,
        ],
    );

    assert_eq!(code, 0);
    let kn_path = dir.join("knowledge/KN-001.json");
    let content: Value = serde_json::from_str(&std::fs::read_to_string(kn_path).unwrap()).unwrap();
    assert_eq!(content["label"].as_str().unwrap().len(), 5000);
    assert_eq!(content["source"]["excerpt"].as_str().unwrap().len(), 10000);
}

// === K2.10: Sequential IDs ===
#[test]
fn k2_10_sequential_ids() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    init_workspace(dir);

    for i in 1..=3 {
        let (json, code) = run_ltp(
            dir,
            &[
                "knowledge",
                "add",
                &format!("Item {}", i),
                "--type",
                "measurement",
                "--source-uri",
                "x",
            ],
        );
        assert_eq!(code, 0);
        assert_eq!(json["data"]["created_knowledge_id"], format!("KN-{:03}", i));
    }
}

// === K2.11: Edit label ===
#[test]
fn k2_11_edit_label() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    init_workspace(dir);

    run_ltp(
        dir,
        &[
            "knowledge",
            "add",
            "Original",
            "--type",
            "measurement",
            "--source-uri",
            "x",
        ],
    );

    let (json, code) = run_ltp(
        dir,
        &["knowledge", "edit", "KN-001", "--label", "Nuevo texto"],
    );

    assert_eq!(code, 0);
    assert_eq!(json["success"], true);
    assert_eq!(json["data"]["label"], "Nuevo texto");

    let kn: Value =
        serde_json::from_str(&std::fs::read_to_string(dir.join("knowledge/KN-001.json")).unwrap())
            .unwrap();
    assert_eq!(kn["label"], "Nuevo texto");
}

// === K2.12: Edit status preserves captured ===
#[test]
fn k2_12_edit_status_preserves_captured() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    init_workspace(dir);

    run_ltp(
        dir,
        &[
            "knowledge",
            "add",
            "Item",
            "--type",
            "measurement",
            "--source-uri",
            "x",
        ],
    );

    let kn_before: Value =
        serde_json::from_str(&std::fs::read_to_string(dir.join("knowledge/KN-001.json")).unwrap())
            .unwrap();
    let captured_before = kn_before["captured"].as_str().unwrap().to_string();

    let (json, code) = run_ltp(dir, &["knowledge", "edit", "KN-001", "--status", "refuted"]);

    assert_eq!(code, 0);
    assert_eq!(json["data"]["status"], "refuted");

    let kn_after: Value =
        serde_json::from_str(&std::fs::read_to_string(dir.join("knowledge/KN-001.json")).unwrap())
            .unwrap();
    assert_eq!(kn_after["captured"].as_str().unwrap(), captured_before);
}

// === K2.13: Edit confidence ===
#[test]
fn k2_13_edit_confidence() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    init_workspace(dir);

    run_ltp(
        dir,
        &[
            "knowledge",
            "add",
            "Item",
            "--type",
            "measurement",
            "--source-uri",
            "x",
        ],
    );

    let (json, code) = run_ltp(dir, &["knowledge", "edit", "KN-001", "--confidence", "low"]);

    assert_eq!(code, 0);
    assert_eq!(json["data"]["confidence"], "low");
}

// === K2.14: Edit add-tag ===
#[test]
fn k2_14_edit_add_tag() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    init_workspace(dir);

    run_ltp(
        dir,
        &[
            "knowledge",
            "add",
            "Item",
            "--type",
            "measurement",
            "--source-uri",
            "x",
        ],
    );

    let (json, code) = run_ltp(
        dir,
        &["knowledge", "edit", "KN-001", "--add-tag", "q2-2026"],
    );

    assert_eq!(code, 0);
    let tags = json["data"]["tags"].as_array().unwrap();
    assert!(tags.contains(&Value::String("q2-2026".to_string())));
}

// === K2.15: Edit rm-tag ===
#[test]
fn k2_15_edit_rm_tag() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    init_workspace(dir);

    run_ltp(
        dir,
        &[
            "knowledge",
            "add",
            "Item",
            "--type",
            "measurement",
            "--source-uri",
            "x",
            "--tags",
            "logistica,proveedor",
        ],
    );

    let (json, code) = run_ltp(
        dir,
        &["knowledge", "edit", "KN-001", "--rm-tag", "logistica"],
    );

    assert_eq!(code, 0);
    let tags = json["data"]["tags"].as_array().unwrap();
    assert!(!tags.contains(&Value::String("logistica".to_string())));
    assert!(tags.contains(&Value::String("proveedor".to_string())));
}

// === K2.16: Edit source ===
#[test]
fn k2_16_edit_source() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    init_workspace(dir);

    run_ltp(
        dir,
        &[
            "knowledge",
            "add",
            "Item",
            "--type",
            "measurement",
            "--source-uri",
            "old://uri",
        ],
    );

    let (_, code) = run_ltp(
        dir,
        &[
            "knowledge",
            "edit",
            "KN-001",
            "--source-uri",
            "new://uri",
            "--source-excerpt",
            "new excerpt",
        ],
    );

    assert_eq!(code, 0);
    let kn: Value =
        serde_json::from_str(&std::fs::read_to_string(dir.join("knowledge/KN-001.json")).unwrap())
            .unwrap();
    assert_eq!(kn["source"]["uri"], "new://uri");
    assert_eq!(kn["source"]["excerpt"], "new excerpt");
}

// === K2.17: Edit non-existent ID ===
#[test]
fn k2_17_edit_not_found() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    init_workspace(dir);

    let (json, code) = run_ltp(dir, &["knowledge", "edit", "KN-999", "--label", "x"]);

    assert_ne!(code, 0);
    assert_eq!(json["errors"][0]["code"], "KNOWLEDGE_NOT_FOUND");
}

// === K2.18: Edit empty label rejected ===
#[test]
fn k2_18_edit_empty_label_rejected() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    init_workspace(dir);

    run_ltp(
        dir,
        &[
            "knowledge",
            "add",
            "Item",
            "--type",
            "measurement",
            "--source-uri",
            "x",
        ],
    );

    let (json, code) = run_ltp(dir, &["knowledge", "edit", "KN-001", "--label", ""]);

    assert_ne!(code, 0);
    assert_eq!(json["errors"][0]["code"], "LABEL_REQUIRED");
}

// === K2.20: Edit add-tag deduplicates ===
#[test]
fn k2_20_edit_add_tag_dedup() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    init_workspace(dir);

    run_ltp(
        dir,
        &[
            "knowledge",
            "add",
            "Item",
            "--type",
            "measurement",
            "--source-uri",
            "x",
            "--tags",
            "a",
        ],
    );

    let (json, code) = run_ltp(dir, &["knowledge", "edit", "KN-001", "--add-tag", "a,a"]);

    assert_eq!(code, 0);
    let tags = json["data"]["tags"].as_array().unwrap();
    let count = tags.iter().filter(|t| t.as_str() == Some("a")).count();
    assert_eq!(count, 1);
}

// === K2.21: Edit rm-tag non-existent is no-op ===
#[test]
fn k2_21_edit_rm_tag_nonexistent() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    init_workspace(dir);

    run_ltp(
        dir,
        &[
            "knowledge",
            "add",
            "Item",
            "--type",
            "measurement",
            "--source-uri",
            "x",
        ],
    );

    let (json, code) = run_ltp(
        dir,
        &["knowledge", "edit", "KN-001", "--rm-tag", "inexistente"],
    );

    assert_eq!(code, 0);
    assert!(json["warnings"]
        .as_array()
        .unwrap()
        .iter()
        .any(|w| w["code"] == "TAG_NOT_FOUND"));
}

// === K2.22: Edit clearing both source fields rejected ===
#[test]
fn k2_22_edit_clear_both_sources_rejected() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    init_workspace(dir);

    run_ltp(
        dir,
        &[
            "knowledge",
            "add",
            "Item",
            "--type",
            "measurement",
            "--source-uri",
            "x",
            "--source-excerpt",
            "y",
        ],
    );

    let (json, code) = run_ltp(
        dir,
        &[
            "knowledge",
            "edit",
            "KN-001",
            "--source-uri",
            "",
            "--source-excerpt",
            "",
        ],
    );

    assert_ne!(code, 0);
    assert_eq!(json["errors"][0]["code"], "SOURCE_REQUIRED");
}

// === K2.23: Edit clear URI but excerpt remains ===
#[test]
fn k2_23_edit_clear_uri_excerpt_remains() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    init_workspace(dir);

    run_ltp(
        dir,
        &[
            "knowledge",
            "add",
            "Item",
            "--type",
            "measurement",
            "--source-uri",
            "x",
            "--source-excerpt",
            "y",
        ],
    );

    let (_, code) = run_ltp(dir, &["knowledge", "edit", "KN-001", "--source-uri", ""]);

    assert_eq!(code, 0);
    let kn: Value =
        serde_json::from_str(&std::fs::read_to_string(dir.join("knowledge/KN-001.json")).unwrap())
            .unwrap();
    assert_eq!(kn["source"]["uri"], Value::Null);
    assert_eq!(kn["source"]["excerpt"], "y");
}

// === K2.26: Rm single item ===
#[test]
fn k2_26_rm_single() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    init_workspace(dir);

    run_ltp(
        dir,
        &[
            "knowledge",
            "add",
            "Item",
            "--type",
            "measurement",
            "--source-uri",
            "x",
        ],
    );

    let (json, code) = run_ltp(dir, &["knowledge", "rm", "KN-001"]);

    assert_eq!(code, 0);
    assert_eq!(json["success"], true);
    assert!(json["data"]["removed"]
        .as_array()
        .unwrap()
        .contains(&Value::String("KN-001".to_string())));
    assert!(!dir.join("knowledge/KN-001.json").exists());
}

// === K2.27: Rm batch ===
#[test]
fn k2_27_rm_batch() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    init_workspace(dir);

    for _ in 0..2 {
        run_ltp(
            dir,
            &[
                "knowledge",
                "add",
                "Item",
                "--type",
                "measurement",
                "--source-uri",
                "x",
            ],
        );
    }

    let (json, code) = run_ltp(dir, &["knowledge", "rm", "KN-001,KN-002"]);

    assert_eq!(code, 0);
    let removed = json["data"]["removed"].as_array().unwrap();
    assert_eq!(removed.len(), 2);
    assert!(!dir.join("knowledge/KN-001.json").exists());
    assert!(!dir.join("knowledge/KN-002.json").exists());
}

// === K2.28: Rm not found ===
#[test]
fn k2_28_rm_not_found() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    init_workspace(dir);

    let (json, code) = run_ltp(dir, &["knowledge", "rm", "KN-999"]);

    assert_ne!(code, 0);
    assert_eq!(json["errors"][0]["code"], "KNOWLEDGE_NOT_FOUND");
}

// === K2.29: Rm batch partial (one exists, one doesn't) ===
#[test]
fn k2_29_rm_batch_partial() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    init_workspace(dir);

    run_ltp(
        dir,
        &[
            "knowledge",
            "add",
            "Item",
            "--type",
            "measurement",
            "--source-uri",
            "x",
        ],
    );

    let (json, code) = run_ltp(dir, &["knowledge", "rm", "KN-001,KN-999"]);

    // Partial success: KN-001 removed, KN-999 reported as error
    assert_eq!(code, 0); // success because at least one was removed
    let removed = json["data"]["removed"].as_array().unwrap();
    assert!(removed.contains(&Value::String("KN-001".to_string())));
    let failed = json["data"]["failed"].as_array().unwrap();
    assert!(failed.contains(&Value::String("KN-999".to_string())));
}

// === K2.30: Rm double delete ===
#[test]
fn k2_30_rm_double_delete() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    init_workspace(dir);

    run_ltp(
        dir,
        &[
            "knowledge",
            "add",
            "Item",
            "--type",
            "measurement",
            "--source-uri",
            "x",
        ],
    );

    run_ltp(dir, &["knowledge", "rm", "KN-001"]);
    let (json, code) = run_ltp(dir, &["knowledge", "rm", "KN-001"]);

    assert_ne!(code, 0);
    assert_eq!(json["errors"][0]["code"], "KNOWLEDGE_NOT_FOUND");
}

// === K2.33: Inspect shows full details ===
#[test]
fn k2_33_inspect() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    init_workspace(dir);

    run_ltp(
        dir,
        &[
            "knowledge",
            "add",
            "Media 18.3 dias",
            "--type",
            "measurement",
            "--source-uri",
            "erp://q2",
            "--source-excerpt",
            "Extracto Q2",
            "--status",
            "verified",
            "--confidence",
            "high",
            "--tags",
            "logistica",
        ],
    );

    let (json, code) = run_ltp(dir, &["knowledge", "inspect", "KN-001"]);

    assert_eq!(code, 0);
    assert_eq!(json["data"]["id"], "KN-001");
    assert_eq!(json["data"]["type"], "measurement");
    assert_eq!(json["data"]["label"], "Media 18.3 dias");
    assert_eq!(json["data"]["status"], "verified");
    assert_eq!(json["data"]["confidence"], "high");
    assert_eq!(json["data"]["source"]["uri"], "erp://q2");
    assert_eq!(json["data"]["source"]["excerpt"], "Extracto Q2");
    assert!(!json["data"]["captured"].as_str().unwrap().is_empty());
    assert!(json["data"]["tags"]
        .as_array()
        .unwrap()
        .contains(&Value::String("logistica".to_string())));
}

// === K2.34: Inspect not found ===
#[test]
fn k2_34_inspect_not_found() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    init_workspace(dir);

    let (json, code) = run_ltp(dir, &["knowledge", "inspect", "KN-999"]);

    assert_ne!(code, 0);
    assert_eq!(json["errors"][0]["code"], "KNOWLEDGE_NOT_FOUND");
}

// === K2.35: List with items ===
#[test]
fn k2_35_list_items() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    init_workspace(dir);

    for i in 1..=3 {
        run_ltp(
            dir,
            &[
                "knowledge",
                "add",
                &format!("Item {}", i),
                "--type",
                "measurement",
                "--source-uri",
                "x",
            ],
        );
    }

    let (json, code) = run_ltp(dir, &["knowledge", "list"]);

    assert_eq!(code, 0);
    assert_eq!(json["data"]["total"], 3);
    let items = json["data"]["items"].as_array().unwrap();
    assert_eq!(items.len(), 3);
    assert_eq!(items[0]["id"], "KN-001");
}

// === K2.36: List empty pool ===
#[test]
fn k2_36_list_empty() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    init_workspace(dir);

    let (json, code) = run_ltp(dir, &["knowledge", "list"]);

    assert_eq!(code, 0);
    assert_eq!(json["data"]["total"], 0);
    assert_eq!(json["data"]["items"].as_array().unwrap().len(), 0);
}

// === K2.37: List filter by type ===
#[test]
fn k2_37_list_filter_type() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    init_workspace(dir);

    run_ltp(
        dir,
        &[
            "knowledge",
            "add",
            "A",
            "--type",
            "measurement",
            "--source-uri",
            "x",
        ],
    );
    run_ltp(
        dir,
        &[
            "knowledge",
            "add",
            "B",
            "--type",
            "testimony",
            "--source-uri",
            "x",
        ],
    );

    let (json, code) = run_ltp(dir, &["knowledge", "list", "--type", "measurement"]);

    assert_eq!(code, 0);
    assert_eq!(json["data"]["total"], 1);
    assert_eq!(json["data"]["items"][0]["type"], "measurement");
}

// === K2.38: List filter by status ===
#[test]
fn k2_38_list_filter_status() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    init_workspace(dir);

    run_ltp(
        dir,
        &[
            "knowledge",
            "add",
            "A",
            "--type",
            "measurement",
            "--source-uri",
            "x",
            "--status",
            "verified",
        ],
    );
    run_ltp(
        dir,
        &[
            "knowledge",
            "add",
            "B",
            "--type",
            "measurement",
            "--source-uri",
            "x",
        ],
    );

    let (json, code) = run_ltp(dir, &["knowledge", "list", "--status", "unverified"]);

    assert_eq!(code, 0);
    assert_eq!(json["data"]["total"], 1);
    assert_eq!(json["data"]["items"][0]["status"], "unverified");
}

// === K2.39: List filter unlinked ===
#[test]
fn k2_39_list_unlinked() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    init_workspace(dir);

    run_ltp(
        dir,
        &[
            "knowledge",
            "add",
            "Unlinked item",
            "--type",
            "measurement",
            "--source-uri",
            "x",
        ],
    );

    let (json, code) = run_ltp(dir, &["knowledge", "list", "--unlinked"]);

    assert_eq!(code, 0);
    assert_eq!(json["data"]["total"], 1);
}

// === K2.40: List filter by tag ===
#[test]
fn k2_40_list_filter_tag() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    init_workspace(dir);

    run_ltp(
        dir,
        &[
            "knowledge",
            "add",
            "A",
            "--type",
            "measurement",
            "--source-uri",
            "x",
            "--tags",
            "logistica",
        ],
    );
    run_ltp(
        dir,
        &[
            "knowledge",
            "add",
            "B",
            "--type",
            "measurement",
            "--source-uri",
            "x",
            "--tags",
            "otro",
        ],
    );

    let (json, code) = run_ltp(dir, &["knowledge", "list", "--tag", "logistica"]);

    assert_eq!(code, 0);
    assert_eq!(json["data"]["total"], 1);
    assert_eq!(json["data"]["items"][0]["label"], "A");
}

// === K2.41: List filter by confidence ===
#[test]
fn k2_41_list_filter_confidence() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    init_workspace(dir);

    run_ltp(
        dir,
        &[
            "knowledge",
            "add",
            "A",
            "--type",
            "measurement",
            "--source-uri",
            "x",
            "--confidence",
            "high",
        ],
    );
    run_ltp(
        dir,
        &[
            "knowledge",
            "add",
            "B",
            "--type",
            "measurement",
            "--source-uri",
            "x",
            "--confidence",
            "low",
        ],
    );

    let (json, code) = run_ltp(dir, &["knowledge", "list", "--confidence", "high"]);

    assert_eq!(code, 0);
    assert_eq!(json["data"]["total"], 1);
    assert_eq!(json["data"]["items"][0]["label"], "A");
}

// === K2.42: List combined filters (AND) ===
#[test]
fn k2_42_list_combined_filters() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    init_workspace(dir);

    run_ltp(
        dir,
        &[
            "knowledge",
            "add",
            "A",
            "--type",
            "measurement",
            "--source-uri",
            "x",
            "--status",
            "verified",
            "--confidence",
            "high",
        ],
    );
    run_ltp(
        dir,
        &[
            "knowledge",
            "add",
            "B",
            "--type",
            "measurement",
            "--source-uri",
            "x",
            "--status",
            "unverified",
            "--confidence",
            "high",
        ],
    );
    run_ltp(
        dir,
        &[
            "knowledge",
            "add",
            "C",
            "--type",
            "testimony",
            "--source-uri",
            "x",
            "--status",
            "verified",
            "--confidence",
            "high",
        ],
    );

    let (json, code) = run_ltp(
        dir,
        &[
            "knowledge",
            "list",
            "--type",
            "measurement",
            "--status",
            "verified",
            "--confidence",
            "high",
        ],
    );

    assert_eq!(code, 0);
    assert_eq!(json["data"]["total"], 1);
    assert_eq!(json["data"]["items"][0]["label"], "A");
}

// === K2.43: List filter with zero matches ===
#[test]
fn k2_43_list_filter_zero_matches() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    init_workspace(dir);

    run_ltp(
        dir,
        &[
            "knowledge",
            "add",
            "A",
            "--type",
            "testimony",
            "--source-uri",
            "x",
        ],
    );

    let (json, code) = run_ltp(dir, &["knowledge", "list", "--type", "measurement"]);

    assert_eq!(code, 0);
    assert_eq!(json["data"]["total"], 0);
}

// === K2.46: Add -> Undo -> Add gives KN-002 (counter doesn't retrocede) ===
#[test]
fn k2_46_undo_counter_monotonic() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    init_workspace(dir);

    run_ltp(
        dir,
        &[
            "knowledge",
            "add",
            "First",
            "--type",
            "measurement",
            "--source-uri",
            "x",
        ],
    );

    run_ltp(dir, &["undo"]);

    let (json, code) = run_ltp(
        dir,
        &[
            "knowledge",
            "add",
            "Second",
            "--type",
            "measurement",
            "--source-uri",
            "x",
        ],
    );

    assert_eq!(code, 0);
    assert_eq!(json["data"]["created_knowledge_id"], "KN-002");
}

// === K2.47: Add -> Undo -> Redo restores item ===
#[test]
fn k2_47_undo_redo_roundtrip() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    init_workspace(dir);

    run_ltp(
        dir,
        &[
            "knowledge",
            "add",
            "Item to undo",
            "--type",
            "measurement",
            "--source-uri",
            "x",
            "--status",
            "verified",
        ],
    );

    assert!(dir.join("knowledge/KN-001.json").exists());

    run_ltp(dir, &["undo"]);
    assert!(!dir.join("knowledge/KN-001.json").exists());

    run_ltp(dir, &["redo"]);
    assert!(dir.join("knowledge/KN-001.json").exists());

    let kn: Value =
        serde_json::from_str(&std::fs::read_to_string(dir.join("knowledge/KN-001.json")).unwrap())
            .unwrap();
    assert_eq!(kn["label"], "Item to undo");
    assert_eq!(kn["status"], "verified");
}
