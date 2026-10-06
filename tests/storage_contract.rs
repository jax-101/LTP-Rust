//! Contrato del trait `Storage`, independiente del backend.
//!
//! `Storage::load_tree` debe devolver el árbol normalizado (ADR-014): la lógica del árbol y
//! de sus edges se deriva de `tree_type`, y los edges de las ramas NBR son siempre
//! `SUFFICIENCY`. `assert_load_tree_normalizes` es el contrato reutilizable: cada backend
//! (hoy `FsStorage`; mañana Turso) debe pasarlo con su propia instancia.

use ltp_engine::link::{Edge, EdgeStatus, Logic, Operator};
use ltp_engine::storage::Storage;
use ltp_engine::tree::{NbrBranch, NodeRef, Tree, TreeLogic, TreeType};
use ltp_engine::workspace::FsStorage;

const ALL_TYPES: [TreeType; 6] = [
    TreeType::Gt,
    TreeType::Crt,
    TreeType::Ec,
    TreeType::Frt,
    TreeType::Prt,
    TreeType::Tt,
];

fn edge(id: &str, from: &str, to: &str, logic: Logic) -> Edge {
    Edge {
        id: id.to_string(),
        from: vec![from.to_string()],
        to: to.to_string(),
        operator: Operator::Single,
        weight: None,
        status: EdgeStatus::Active,
        logic,
        assumptions: vec![],
    }
}

/// Árbol cuya lógica contradice su tipo: tronco con la lógica opuesta y rama NBR en
/// `NECESSITY` (nunca válido).
fn corrupt_tree(id: &str, tree_type: TreeType) -> Tree {
    let (wrong_tree, wrong_edge) = match tree_type.logic() {
        TreeLogic::Sufficiency => (TreeLogic::Necessity, Logic::Necessity),
        TreeLogic::Necessity => (TreeLogic::Sufficiency, Logic::Sufficiency),
    };
    Tree {
        id: id.to_string(),
        name: format!("corrupt {tree_type:?}"),
        tree_type,
        logic: wrong_tree,
        nodes: ["A-001", "B-001"]
            .iter()
            .map(|r| NodeRef {
                node_ref: (*r).to_string(),
                role: None,
            })
            .collect(),
        edges: vec![edge("LINK-001", "A-001", "B-001", wrong_edge)],
        macro_edges: vec![],
        feedback_edges: vec![],
        nbr_branches: vec![NbrBranch {
            id: "NBR-001".to_string(),
            source_node: "B-001".to_string(),
            edges: vec![edge("LINK-002", "B-001", "UDE-001", Logic::Necessity)],
            trim_injection: None,
        }],
    }
}

/// Contrato: cualquier backend devuelve desde `load_tree` el árbol ya normalizado,
/// para todos los tipos de árbol.
fn assert_load_tree_normalizes(storage: &dyn Storage) {
    for (i, tree_type) in ALL_TYPES.into_iter().enumerate() {
        let id = format!("tree-contract-{i}");
        storage.save_tree(&corrupt_tree(&id, tree_type)).unwrap();

        let loaded = storage.load_tree(&id).unwrap();
        let expected = tree_type.logic();
        assert_eq!(loaded.logic, expected, "{tree_type:?}: tree logic");
        assert!(
            loaded
                .edges
                .iter()
                .all(|e| e.logic == Logic::from(expected)),
            "{tree_type:?}: trunk edges must inherit {expected:?}"
        );
        assert!(
            loaded
                .nbr_branches
                .iter()
                .flat_map(|b| &b.edges)
                .all(|e| e.logic == Logic::Sufficiency),
            "{tree_type:?}: NBR edges must be SUFFICIENCY"
        );
    }
}

#[test]
fn fs_storage_load_tree_normalizes() {
    let tmp = tempfile::tempdir().unwrap();
    let storage = FsStorage::new(tmp.path().to_path_buf());
    storage.init_workspace("contract").unwrap();
    assert_load_tree_normalizes(&storage);
}

/// Específico de `FsStorage`: normalizar al leer no reescribe el fichero (ADR-009, undo
/// por checksum).
#[test]
fn fs_storage_load_tree_does_not_write_back() {
    let tmp = tempfile::tempdir().unwrap();
    let storage = FsStorage::new(tmp.path().to_path_buf());
    storage.init_workspace("contract").unwrap();
    storage
        .save_tree(&corrupt_tree("tree-legacy", TreeType::Gt))
        .unwrap();
    let path = tmp.path().join("trees/tree-legacy.json");
    let before = std::fs::read(&path).unwrap();

    let loaded = storage.load_tree("tree-legacy").unwrap();
    assert_eq!(loaded.logic, TreeLogic::Necessity);
    assert_eq!(
        std::fs::read(&path).unwrap(),
        before,
        "load_tree wrote to disk"
    );
}
