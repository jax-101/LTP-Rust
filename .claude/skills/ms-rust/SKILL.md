---
name: ms-rust
description: Estandares arquitectonicos y de calidad para desarrollo en Rust dentro de ltp-engine.
---

# Directrices de Rust para ltp-engine

## Errores y Seguridad
- Usa `thiserror` para gestion de errores en modulos internos.
- Prohibido el uso de `.unwrap()` y `.expect()` en codigo de produccion.

## Determinismo
- Mantiene determinismo en la serializacion JSON usando `BTreeMap` en las estructuras de datos.

## Documentacion
- Agrega comentarios de documentacion `///` a todos los items publicos (`pub`).

## Type-First
- Definir primero structs, enums y traits antes de implementar la logica interna. El sistema de tipos guia el diseno.

## Borrow Checker
- Prohibido usar `.clone()` excesivos o `Arc<Mutex<T>>` como parche para esquivar al borrow checker. Resolver conflictos de ownership con lifetimes explicitos o reestructuracion del codigo.

## Depuracion
- Prohibido dejar macros `dbg!()` o `println!()` en codigo final. Usar logging estructurado si se requiere tracing.
