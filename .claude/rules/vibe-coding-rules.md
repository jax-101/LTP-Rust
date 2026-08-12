# Reglas de Flujo y Desarrollo

- **Metodologia RPI (Research-Plan-Implement)**: Para cada tarea o fase, primero investiga sin editar, genera un plan en `.claude/plans/`, solicita aprobacion y luego implementa en un contexto fresco.
- **Research — Fuentes obligatorias**: Antes de redactar cualquier plan en `.claude/plans/`, es OBLIGATORIO consultar los archivos de especificacion relevantes al scope de la tarea:
  - `ENGINE_SPEC.md` — Comandos, esquemas JSON, contrato de salida.
  - `ADR.md` — Decisiones de arquitectura vigentes.
  - `CLR_SPEC.md` — Categorias de Legitimacion Logica (si la tarea involucra validacion o linting).
  
  Esto garantiza que el plan refleje las decisiones ya tomadas y no contradiga invariantes del proyecto.
- **Verificacion**: Nunca des por completada una tarea sin haber pasado la secuencia completa:
  1. `cargo check --all-targets --all-features`
  2. `cargo clippy --all-targets --all-features -- -D warnings`
  3. `cargo test --workspace`
  4. `cargo fmt --all -- --check`
- **Registro de Progreso**: Al completar cada paquete de trabajo o UAT, actualiza el estado en `PROGRESS.md` calculando el factor de escala y el avance global segun las reglas definidas en dicho archivo.
