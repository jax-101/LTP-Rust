//! Contract snapshot tests — golden fixtures del `CommandOutput` JSON.
//!
//! Cada `contract/*.json` es un `CommandOutput` canónico real, capturado del
//! binario `ltp` (no de funciones de librería) ejecutando una secuencia fija
//! sobre un workspace temporal. Como los IDs son secuenciales por tipo (ADR-001,
//! determinismo absoluto) y los tree-ids derivan del slug del nombre, re-ejecutar
//! la secuencia produce goldens byte-estables.
//!
//! Un cambio de *shape* de salida rompe este test con un diff exacto, forzando una
//! decisión consciente de versión (RELEASE_POLICY §1). Testea **forma**, no
//! semántica (ADR-001: la verdad lógica es del agente LLM, no del motor).
//!
//! Regenerar (cambio de contrato intencional):
//! ```text
//! UPDATE_GOLDEN=1 cargo test --test contract
//! ```
//! Luego revisar el diff de los goldens a ojo y anotar el bump SemVer.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;

// ─── Infra: spawn del binario real ──────────────────────────────────────────

fn ltp_bin() -> String {
    env!("CARGO_BIN_EXE_ltp").to_string()
}

// Ejecuta `ltp <args>` con `cwd = dir` y devuelve (stdout parseado, exit code).
// Reutiliza el patrón de tests/e2e.rs: snapshotamos exactamente lo que la UI ve
// (arg parsing, resolución de cwd, stdout), no un atajo por librería.
fn run_ltp(dir: &Path, args: &[&str]) -> (Value, i32) {
    let output = Command::new(ltp_bin())
        .args(args)
        .current_dir(dir)
        .output()
        .expect("failed to execute ltp binary");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let code = output.status.code().unwrap_or(-1);
    let json: Value = serde_json::from_str(&stdout).unwrap_or_else(|_| {
        panic!(
            "Failed to parse JSON.\nargs: {:?}\nstdout: {}\nstderr: {}",
            args,
            stdout,
            String::from_utf8_lossy(&output.stderr)
        )
    });
    (json, code)
}

// Ejecuta un comando de mutación del fixture y aborta si no tuvo éxito.
fn run_ok(dir: &Path, args: &[&str]) {
    let (json, code) = run_ltp(dir, args);
    assert!(
        code == 0 && json["success"].as_bool() == Some(true),
        "fixture step failed (code {code}): {args:?}\n{json:#}"
    );
}

// ─── Fixture determinista ─────────────────────────────────────────────────────

// Construye el workspace-fixture ejecutando una secuencia fija de comandos.
// Devuelve el tree-id (determinista) del CRT creado.
//
// Grafo resultante (CRT):
//   RC-001 ─┐(AND)
//           ├─► INT-001 ──(SINGLE)──► UDE-001   (UDE-001 dispara CLR#4)
//   RC-002 ─┘
//   KN-001 (measurement) ──supports──► INT-001
fn build_fixture(dir: &Path) -> String {
    run_ok(dir, &["init", "--name", "ContractFixture"]);

    // Nodos de tipos variados → IDs secuenciales por tipo.
    run_ok(dir, &["node", "add", "Root cause one", "--type", "RC"]);
    run_ok(dir, &["node", "add", "Root cause two", "--type", "RC"]);
    run_ok(
        dir,
        &["node", "add", "Intermediate effect", "--type", "INT"],
    );
    run_ok(dir, &["node", "add", "Undesirable effect", "--type", "UDE"]);

    // CRT: el tree-id deriva del slug del nombre (determinista).
    let (tree_json, code) = run_ltp(dir, &["tree", "new", "crt", "Contract CRT"]);
    assert_eq!(code, 0, "tree new failed: {tree_json:#}");
    let tree = tree_json["data"]["id"]
        .as_str()
        .expect("tree new must return data.id")
        .to_string();

    for node in ["RC-001", "RC-002", "INT-001", "UDE-001"] {
        run_ok(dir, &["tree", "attach", "--tree", &tree, "--node", node]);
    }

    // RC-001 + RC-002 ──AND──► INT-001 ; INT-001 ──SINGLE──► UDE-001.
    run_ok(
        dir,
        &[
            "link",
            "connect",
            "--tree",
            &tree,
            "--from",
            "RC-001",
            "--to",
            "INT-001",
            "--operator",
            "AND",
        ],
    );
    run_ok(
        dir,
        &[
            "link",
            "connect",
            "--tree",
            &tree,
            "--from",
            "RC-002",
            "--to",
            "INT-001",
            "--operator",
            "AND",
        ],
    );
    run_ok(
        dir,
        &[
            "link", "connect", "--tree", &tree, "--from", "INT-001", "--to", "UDE-001",
        ],
    );

    // Knowledge item enlazado (para el caso --show-knowledge).
    run_ok(
        dir,
        &[
            "knowledge",
            "add",
            "Lead time measured at 18 days",
            "--type",
            "measurement",
            "--source-excerpt",
            "ERP report Q2",
        ],
    );
    run_ok(
        dir,
        &[
            "knowledge",
            "link",
            "KN-001",
            "--to",
            "INT-001",
            "--relation",
            "supports",
        ],
    );

    tree
}

// ─── Normalización de campos volátiles (plan §3.4) ────────────────────────────

// Redacta in-place los campos no deterministas que romperían el snapshot sin que
// cambie el contrato:
//   - `workspace` (nivel raíz): nombre/ruta del workspace → "<WORKSPACE>".
//   - claves de timestamp (ISO 8601 de chrono) → "<TIMESTAMP>". El conjunto
//     inicial no las expone, pero se redactan por nombre de clave para blindar
//     goldens futuros (p. ej. un `node inspect` con `created_at`, o el contexto
//     `timestamp` de un error de lock).
// Redacta el **valor**, nunca la clave: la presencia y el tipo del campo siguen
// siendo parte del contrato verificado.
fn redact_volatile(value: &mut Value) {
    match value {
        Value::Object(map) => {
            for (key, val) in map.iter_mut() {
                match key.as_str() {
                    "workspace" => *val = Value::String("<WORKSPACE>".to_string()),
                    "created_at" | "updated_at" | "timestamp" => {
                        *val = Value::String("<TIMESTAMP>".to_string())
                    }
                    _ => redact_volatile(val),
                }
            }
        }
        Value::Array(items) => items.iter_mut().for_each(redact_volatile),
        _ => {}
    }
}

// ─── Comparación / regeneración de goldens ────────────────────────────────────

fn contract_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("contract")
}

// Compara el output vivo (ya redactado) contra el golden, o lo reescribe si
// UPDATE_GOLDEN está activo. Devuelve Err con un diff legible en caso de deriva.
fn check_or_update_golden(name: &str, live: &Value, update: bool) -> Result<(), String> {
    let path = contract_dir().join(format!("{name}.json"));
    let serialized = serde_json::to_string_pretty(live).map_err(|e| e.to_string())?;

    if update {
        fs::write(&path, format!("{serialized}\n"))
            .map_err(|e| format!("no se pudo escribir {name}.json: {e}"))?;
        return Ok(());
    }

    let golden_raw = fs::read_to_string(&path).map_err(|_| {
        format!(
            "falta golden {name}.json — genera con `UPDATE_GOLDEN=1 cargo test --test contract`"
        )
    })?;
    let golden: Value = serde_json::from_str(&golden_raw)
        .map_err(|e| format!("golden {name}.json inválido: {e}"))?;

    // Comparación de Value (independiente del orden de claves) — plan §Verde.
    if &golden == live {
        Ok(())
    } else {
        Err(format!(
            "── DERIVA DE CONTRATO en {name}.json ──\n\
             --- golden (esperado) ---\n{}\n\
             --- vivo (actual) ---\n{serialized}\n\
             Si el cambio es intencional: `UPDATE_GOLDEN=1 cargo test --test contract`, \
             revisa el diff y ajusta el bump SemVer (RELEASE_POLICY §1).",
            serde_json::to_string_pretty(&golden).unwrap_or_default()
        ))
    }
}

// ─── Test ─────────────────────────────────────────────────────────────────────

#[test]
fn contract_output_snapshots() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let dir = tmp.path();
    let tree = build_fixture(dir);

    // Conjunto inicial de casos (crecer bajo demanda). Los read-only y el error
    // atómico (self-loop: falla y libera el lock sin escribir) no mutan el fixture,
    // así que comparten un único workspace.
    let cases: [(&str, Vec<&str>); 5] = [
        // nodos {id, role, incoming_edges[], outgoing_edges[]}; SIN feedback_edges.
        ("tree_walk", vec!["tree", "walk", &tree]),
        // + campo opcional knowledge{supports,contradicts,contextualizes}.
        (
            "tree_walk_knowledge",
            vec!["tree", "walk", &tree, "--show-knowledge"],
        ),
        // tree_type (no `type`), enums minúscula, node_count/edge_count, logic.
        ("tree_list", vec!["tree", "list"]),
        // graph_health + warnings CLR anidados en data.details[] (contexto aplanado).
        ("validate", vec!["validate"]),
        // errors[] = {code, detail, ...contexto aplanado} (aquí node_id, al nivel raíz).
        // Self-loop de reserva: falla de forma atómica (libera el lock antes de
        // escribir) y con contenido determinista — no como cycle_path, cuya
        // rotación no es determinista entre ejecuciones (ver contract/README.md).
        (
            "error_flattened_context",
            vec![
                "macro",
                "add",
                "--tree",
                &tree,
                "--from",
                "RC-001",
                "--to",
                "RC-001",
                "--label",
                "Self loop",
            ],
        ),
    ];

    let update = std::env::var_os("UPDATE_GOLDEN").is_some();
    let mut failures: Vec<String> = Vec::new();

    for (name, args) in &cases {
        let (mut live, _code) = run_ltp(dir, args);
        redact_volatile(&mut live);
        if let Err(msg) = check_or_update_golden(name, &live, update) {
            failures.push(msg);
        }
    }

    assert!(failures.is_empty(), "\n{}", failures.join("\n\n"));
}
