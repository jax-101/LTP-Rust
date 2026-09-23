# RELEASE_POLICY.md

Política de versionado y releases de `ltp-engine`. **Objetivo único: cero ambigüedad** entre el binario en ejecución, el commit de git del que salió y la documentación que le corresponde.

## El triángulo que cierra

```
        binario en ejecución
              │  reporta  version + SHA
              ▼
        commit git  ──(tag vX.Y.Z)──  documentación (CHANGELOG + specs en ese commit)
```

Desde cualquier binario en marcha se obtiene su versión y su SHA (`ltp --version` o el `initialize` del MCP). **El SHA es el ancla**: `git checkout <sha>` reproduce el código y los docs exactos. El tag da el nombre humano. El `CHANGELOG` dice qué hace esa versión.

## 1. Versionado (SemVer anclado al contrato)

La "API pública" del motor es el **contrato MCP/CLI**: nombres de tools, sus parámetros, el esquema JSON de salida (`CommandOutput`) y los códigos de error/warning.

| Incremento | Cuándo | Ejemplos |
|-----------|--------|----------|
| **MAJOR** (`x`) | Rompe el contrato | Eliminar/renombrar un tool o param; cambiar el shape del JSON de salida; cambiar el significado de un error code; cambiar el formato de IDs |
| **MINOR** (`y`) | Aditivo y retrocompatible | Nuevo tool; param opcional nuevo; campo nuevo en la salida; warning/error code nuevo |
| **PATCH** (`z`) | Sin cambio de contrato | Bugfix; corrección de descripción/doc; refactor interno; performance |

- Semántica **estricta desde `0.x`** (no aplicamos la cláusula laxa de SemVer para `0.x`).
- `1.0.0` se reserva para "contrato congelado / estable".
- **Una sola versión** para todo el motor (el workspace); ambos binarios (`ltp`, `ltp-mcp`) la comparten.

## 2. Provenance embebida en el binario

`build.rs` embebe, en tiempo de compilación, el SHA corto de git y un flag `dirty`:

- **Formato**: SemVer build metadata → `MAJOR.MINOR.PATCH+<sha>` o `MAJOR.MINOR.PATCH+<sha>.dirty`.
  - Ej.: `0.2.0+a1b2c3d4`, `0.2.0+a1b2c3d4.dirty`.
- Expuesto **idéntico** por:
  - CLI: `ltp --version`
  - MCP: `initialize` → `serverInfo.version`
- Si git no está disponible (build desde tarball), el SHA es `unknown` → `0.2.0+unknown`.

**Invariantes:**

- La provenance vive **solo** en la identificación del binario, **jamás** en el JSON canónico de los grafos (`nodes/`, `trees/`), que debe permanecer determinista (ADR-001).
- **No** se embebe timestamp de build, para preservar builds reproducibles (mismo commit limpio → mismo binario).

**Caveat del flag `dirty`** (honesto): por el caché incremental de Cargo, `build.rs` solo se reevalúa cuando cambian `.git/HEAD` o `.git/index`. Una edición del árbol de trabajo *después* de compilar puede dejar el flag desactualizado en builds de desarrollo. **No afecta a los releases**: un release se compila desde un árbol limpio y taggeado (`cargo build --release`), por lo que reporta el SHA exacto sin `dirty`.

## 3. Tags de git

- Cada release lleva un **tag anotado** `vX.Y.Z` sobre el commit de release.
- El SHA embebido en el binario debe corresponder a (o descender directamente de) ese tag.

## 4. CHANGELOG

- `CHANGELOG.md` en la raíz, formato [Keep a Changelog](https://keepachangelog.com/) + SemVer.
- Es el "qué cambió por versión" que consume la UI. Complementa (no sustituye) a `ADR.md` (por qué), `PROGRESS.md` (telemetría) y `ENGINE_SPEC.md` (contrato).
- Cada entrada agrupa cambios en `Added` / `Changed` / `Deprecated` / `Removed` / `Fixed`.

## 5. Checklist de release

1. Aterrizar todos los cambios de la versión en `main`.
2. Gate verde:
   - `cargo check --all-targets --all-features`
   - `cargo clippy --all-targets --all-features -- -D warnings`
   - `cargo test --workspace`
   - `cargo fmt --all -- --check`
3. Decidir el incremento (MAJOR/MINOR/PATCH) según el diff de contrato.
4. Actualizar `version` en `Cargo.toml`.
5. Mover `[Unreleased]` → `[X.Y.Z] - AAAA-MM-DD` en `CHANGELOG.md`.
6. Commit `chore(release): vX.Y.Z`.
7. Tag anotado: `git tag -a vX.Y.Z -m "vX.Y.Z — <resumen>"`.
8. `git push --follow-tags`.

**Un build `dirty` nunca es un release.**

> **Nota de bootstrap**: el primer release bajo esta política (`v0.2.0`) introduce el propio sistema de versionado/provenance en un único commit. A partir de ahí se sigue el checklist de dos pasos (aterrizar features → commit `chore(release)`).

## Cómo identificar un binario (para consumidores / UI)

- **CLI**: `ltp --version` → `ltp 0.2.0+a1b2c3d4`.
- **MCP**: enviar `initialize`, leer `result.serverInfo.version` → `0.2.0+a1b2c3d4`.
- **Feature-gating**: parsear el core antes de `+`; `>= 0.2.0` ⇒ ciclo de vida de la flecha larga (`macro add/expand/promote`) disponible.
- **Reproducir código/docs exactos**: `git checkout <sha>` (el fragmento tras `+`, quitando `.dirty` si aparece).
