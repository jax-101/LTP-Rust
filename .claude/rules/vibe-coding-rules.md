# Reglas de Flujo y Desarrollo

- **Metodologia RPI (Research-Plan-Implement)**: Para cada tarea o fase, primero investiga sin editar, genera un plan en `.claude/plans/`, solicita aprobacion y luego implementa en un contexto fresco.
- **Verificacion**: Nunca des por completada una tarea sin haber pasado la secuencia completa:
  1. `cargo check --all-targets --all-features`
  2. `cargo clippy --all-targets --all-features -- -D warnings`
  3. `cargo test --workspace`
  4. `cargo fmt --all -- --check`
- **Registro de Progreso**: Al completar cada paquete de trabajo o UAT, actualiza el estado en `PROGRESS.md` calculando el factor de escala y el avance global segun las reglas definidas en dicho archivo.
