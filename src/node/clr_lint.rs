use crate::output::OutputWarning;

/// Conjunctions that suggest mixed causality in a single entity (CLR #2).
const CAUSAL_CONJUNCTIONS: &[&str] = &["porque", "in order to", "because", " para ", " y "];

/// Run CLR#2 linter: detect causal conjunctions in a node label.
///
/// Returns warnings for each conjunction found. These are non-blocking —
/// the node is still created/updated regardless.
pub fn lint_clr2(label: &str) -> Vec<OutputWarning> {
    let lower = label.to_lowercase();
    let mut warnings = Vec::new();

    for &conjunction in CAUSAL_CONJUNCTIONS {
        if lower.contains(conjunction) {
            let warning = OutputWarning::new(
                "CLR2_CONJUNCTION_DETECTED",
                format!(
                    "Causal conjunction detected: '{}'. Consider splitting into separate entities.",
                    conjunction.trim()
                ),
            )
            .with_context("label", serde_json::Value::String(label.to_string()));
            warnings.push(warning);
        }
    }

    warnings
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_porque() {
        let warnings = lint_clr2("Vendemos poco porque no hay marketing");
        assert_eq!(warnings.len(), 1);
        assert_eq!(warnings[0].code, "CLR2_CONJUNCTION_DETECTED");
    }

    #[test]
    fn detects_because() {
        let warnings = lint_clr2("Sales drop because of bad logistics");
        assert_eq!(warnings.len(), 1);
    }

    #[test]
    fn detects_in_order_to() {
        let warnings = lint_clr2("We hire more in order to increase capacity");
        assert_eq!(warnings.len(), 1);
    }

    #[test]
    fn clean_label_no_warnings() {
        let warnings = lint_clr2("El tiempo de entrega supera 15 días");
        assert!(warnings.is_empty());
    }

    #[test]
    fn para_requires_spaces() {
        let warnings = lint_clr2("Paralelismo de procesos");
        assert!(warnings.is_empty());
    }
}
