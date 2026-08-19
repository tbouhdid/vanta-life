# VANTA Life — Alpha 0.1

VANTA Life is a local-first Personal Decision OS. It combines a current life
state, strategic goals, and available actions into an explainable deterministic
Next Best Action, then records execution and outcomes for future review.

## Architecture

```text
React UI → Tauri commands → application services → SQLite repository
                                      ↓
                         deterministic Decision Engine
                                      ↓
                            optional AI provider adapter
```

The Decision Engine is pure Rust and never depends on React, SQLite, or an AI
provider. The AI boundary is advisory: **GPT proposes; VANTA decides.**

## Local data and privacy

VANTA is single-user and local-first. On Windows, SQLite is created at:

```text
%LOCALAPPDATA%\com.vantasystems.life\vanta-life.sqlite3
```

Migrations are applied automatically. Profile, check-ins, goals, actions,
decisions, executions, outcomes, memories, and chat history remain local.
OpenAI keys are stored only in Windows Credential Manager through the keyring
adapter, never in SQLite, source control, or the frontend bundle.

## AI mode

OpenAI uses the Responses API through a replaceable `AiProvider` adapter. It
receives a bounded ContextBuilder snapshot and controlled read-only tools. With
AI disabled or unconfigured, onboarding, decisions, actions, outcomes, history,
memory extraction, and analytics continue to work normally.

## Development

From `desktop`:

```powershell
npm.cmd run build
cargo fmt --check --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path src-tauri/Cargo.toml
cargo check --manifest-path src-tauri/Cargo.toml
npm.cmd run tauri -- dev
```

`npm.cmd` avoids a PowerShell execution-policy issue that can block `npm.ps1`.

## Release build

```powershell
npm.cmd run tauri -- build
```

The Tauri configuration uses product name `VANTA Life`, identifier
`com.vantasystems.life`, and version `0.1.0-alpha`.

## Testing

Rust tests cover the decision engine, persistence migrations, CRUD, active
execution restoration, AI provider failure boundaries, structured AI parsing,
memory evidence thresholds, and analytics no-data behavior. Frontend TypeScript
is compiled as part of `npm.cmd run build`.
