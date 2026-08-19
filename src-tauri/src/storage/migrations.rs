use chrono::Utc;
use rusqlite::{Connection, OptionalExtension};

use super::repository::StorageError;

const MIGRATIONS: &[(i64, &str)] = &[
    (
        1,
        r#"
    CREATE TABLE IF NOT EXISTS profile (
        id TEXT PRIMARY KEY NOT NULL,
        name TEXT NOT NULL,
        created_at TEXT NOT NULL,
        updated_at TEXT NOT NULL
    );

    CREATE TABLE IF NOT EXISTS settings (
        id INTEGER PRIMARY KEY CHECK (id = 1),
        start_week_day TEXT NOT NULL,
        default_available_minutes INTEGER NOT NULL CHECK (default_available_minutes >= 0 AND default_available_minutes <= 1440),
        ai_enabled INTEGER NOT NULL CHECK (ai_enabled IN (0, 1)),
        api_configuration_status TEXT NOT NULL
    );

    CREATE TABLE IF NOT EXISTS life_state_entries (
        id TEXT PRIMARY KEY NOT NULL,
        timestamp TEXT NOT NULL,
        energy REAL NOT NULL CHECK (energy >= 0 AND energy <= 10),
        focus REAL NOT NULL CHECK (focus >= 0 AND focus <= 10),
        stress REAL NOT NULL CHECK (stress >= 0 AND stress <= 10),
        sleep_hours REAL NOT NULL CHECK (sleep_hours >= 0 AND sleep_hours <= 24),
        available_minutes INTEGER NOT NULL CHECK (available_minutes >= 0 AND available_minutes <= 1440)
    );

    CREATE TABLE IF NOT EXISTS goals (
        id TEXT PRIMARY KEY NOT NULL,
        title TEXT NOT NULL,
        description TEXT NOT NULL DEFAULT '',
        priority REAL NOT NULL CHECK (priority >= 0 AND priority <= 10),
        active INTEGER NOT NULL CHECK (active IN (0, 1)),
        created_at TEXT NOT NULL,
        completed_at TEXT NULL
    );

    CREATE TABLE IF NOT EXISTS actions (
        id TEXT PRIMARY KEY NOT NULL,
        title TEXT NOT NULL,
        description TEXT NOT NULL DEFAULT '',
        goal_id TEXT NULL REFERENCES goals(id) ON DELETE SET NULL,
        impact REAL NOT NULL CHECK (impact >= 0 AND impact <= 10),
        urgency REAL NOT NULL CHECK (urgency >= 0 AND urgency <= 10),
        goal_alignment REAL NOT NULL CHECK (goal_alignment >= 0 AND goal_alignment <= 10),
        energy_required REAL NOT NULL CHECK (energy_required >= 0 AND energy_required <= 10),
        duration_minutes INTEGER NOT NULL CHECK (duration_minutes >= 1 AND duration_minutes <= 1440),
        status TEXT NOT NULL CHECK (status IN ('open', 'completed')),
        created_at TEXT NOT NULL
    );

    CREATE TABLE IF NOT EXISTS decisions (
        id TEXT PRIMARY KEY NOT NULL,
        timestamp TEXT NOT NULL,
        selected_action_id TEXT NULL,
        score REAL NOT NULL,
        feasible INTEGER NOT NULL CHECK (feasible IN (0, 1)),
        reason TEXT NOT NULL,
        life_state_snapshot TEXT NOT NULL,
        ranking_snapshot TEXT NOT NULL
    );

    CREATE TABLE IF NOT EXISTS executions (
        id TEXT PRIMARY KEY NOT NULL,
        action_id TEXT NOT NULL,
        action_title TEXT NOT NULL,
        decision_id TEXT NULL,
        decision_score REAL NOT NULL,
        started_at TEXT NOT NULL,
        ended_at TEXT NULL,
        status TEXT NOT NULL CHECK (status IN ('in_progress', 'completed', 'abandoned')),
        energy_before REAL NOT NULL CHECK (energy_before >= 0 AND energy_before <= 10)
    );

    CREATE TABLE IF NOT EXISTS outcomes (
        id TEXT PRIMARY KEY NOT NULL,
        execution_id TEXT NOT NULL REFERENCES executions(id) ON DELETE CASCADE,
        completed INTEGER NOT NULL CHECK (completed IN (0, 1)),
        abandoned INTEGER NOT NULL CHECK (abandoned IN (0, 1)),
        result_quality REAL NULL CHECK (result_quality IS NULL OR (result_quality >= 0 AND result_quality <= 10)),
        difficulty REAL NULL CHECK (difficulty IS NULL OR (difficulty >= 0 AND difficulty <= 10)),
        energy_before REAL NOT NULL CHECK (energy_before >= 0 AND energy_before <= 10),
        energy_after REAL NULL CHECK (energy_after IS NULL OR (energy_after >= 0 AND energy_after <= 10)),
        actual_duration_minutes INTEGER NOT NULL CHECK (actual_duration_minutes >= 0),
        created_at TEXT NOT NULL
    );

    CREATE TABLE IF NOT EXISTS memories (
        id TEXT PRIMARY KEY NOT NULL,
        type TEXT NOT NULL,
        content TEXT NOT NULL,
        importance REAL NOT NULL CHECK (importance >= 0 AND importance <= 10),
        source TEXT NOT NULL,
        created_at TEXT NOT NULL,
        last_used_at TEXT NULL
    );

    CREATE TABLE IF NOT EXISTS chat_messages (
        id TEXT PRIMARY KEY NOT NULL,
        role TEXT NOT NULL CHECK (role IN ('user', 'assistant', 'system')),
        content TEXT NOT NULL,
        timestamp TEXT NOT NULL
    );

    CREATE INDEX IF NOT EXISTS idx_life_state_entries_timestamp ON life_state_entries(timestamp DESC);
    CREATE INDEX IF NOT EXISTS idx_goals_active_priority ON goals(active, priority DESC);
    CREATE INDEX IF NOT EXISTS idx_actions_status_goal ON actions(status, goal_id);
    CREATE INDEX IF NOT EXISTS idx_decisions_timestamp ON decisions(timestamp DESC);
    CREATE INDEX IF NOT EXISTS idx_executions_status_started_at ON executions(status, started_at DESC);
    CREATE INDEX IF NOT EXISTS idx_outcomes_created_at ON outcomes(created_at DESC);
    CREATE INDEX IF NOT EXISTS idx_chat_messages_timestamp ON chat_messages(timestamp DESC);
    CREATE UNIQUE INDEX IF NOT EXISTS idx_one_execution_in_progress
        ON executions(status) WHERE status = 'in_progress';
    "#,
    ),
    (
        2,
        r#"
    CREATE INDEX IF NOT EXISTS idx_memories_importance_created_at
        ON memories(importance DESC, created_at DESC);
    "#,
    ),
    // Alpha 0.1 evolves the initial persistence prototype without throwing
    // away existing local data. The `actions` table is rebuilt because SQLite
    // cannot expand its CHECK constraint in place.
    (
        3,
        r#"
    ALTER TABLE profile ADD COLUMN onboarding_completed INTEGER NOT NULL DEFAULT 1
        CHECK (onboarding_completed IN (0, 1));
    ALTER TABLE profile ADD COLUMN default_available_minutes INTEGER NOT NULL DEFAULT 120
        CHECK (default_available_minutes >= 0 AND default_available_minutes <= 1440);

    ALTER TABLE life_state_entries ADD COLUMN optional_note TEXT NULL;
    ALTER TABLE goals ADD COLUMN updated_at TEXT NULL;
    UPDATE goals SET updated_at = created_at WHERE updated_at IS NULL;

    DROP INDEX IF EXISTS idx_actions_status_goal;
    ALTER TABLE actions RENAME TO actions_legacy;
    CREATE TABLE actions (
        id TEXT PRIMARY KEY NOT NULL,
        title TEXT NOT NULL,
        description TEXT NOT NULL DEFAULT '',
        goal_id TEXT NULL REFERENCES goals(id) ON DELETE SET NULL,
        impact REAL NOT NULL CHECK (impact >= 0 AND impact <= 10),
        urgency REAL NOT NULL CHECK (urgency >= 0 AND urgency <= 10),
        goal_alignment REAL NOT NULL CHECK (goal_alignment >= 0 AND goal_alignment <= 10),
        energy_required REAL NOT NULL CHECK (energy_required >= 0 AND energy_required <= 10),
        duration_minutes INTEGER NOT NULL CHECK (duration_minutes >= 1 AND duration_minutes <= 1440),
        status TEXT NOT NULL CHECK (status IN ('available', 'in_progress', 'completed', 'archived')),
        created_at TEXT NOT NULL,
        updated_at TEXT NOT NULL
    );
    INSERT INTO actions (
        id, title, description, goal_id, impact, urgency, goal_alignment,
        energy_required, duration_minutes, status, created_at, updated_at
    )
    SELECT id, title, description, goal_id, impact, urgency, goal_alignment,
           energy_required, duration_minutes,
           CASE status WHEN 'completed' THEN 'completed' ELSE 'available' END,
           created_at, created_at
    FROM actions_legacy;
    DROP TABLE actions_legacy;
    CREATE INDEX idx_actions_status_goal ON actions(status, goal_id);

    ALTER TABLE decisions ADD COLUMN deterministic_reason TEXT NULL;
    UPDATE decisions SET deterministic_reason = reason WHERE deterministic_reason IS NULL;
    ALTER TABLE decisions ADD COLUMN ai_review_available INTEGER NOT NULL DEFAULT 0
        CHECK (ai_review_available IN (0, 1));
    ALTER TABLE decisions ADD COLUMN ai_review_snapshot TEXT NULL;

    ALTER TABLE outcomes ADD COLUMN optional_note TEXT NULL;

    ALTER TABLE memories ADD COLUMN category TEXT NULL;
    ALTER TABLE memories ADD COLUMN statement TEXT NULL;
    ALTER TABLE memories ADD COLUMN confidence REAL NOT NULL DEFAULT 0
        CHECK (confidence >= 0 AND confidence <= 1);
    ALTER TABLE memories ADD COLUMN active INTEGER NOT NULL DEFAULT 1
        CHECK (active IN (0, 1));
    UPDATE memories SET category = type, statement = content WHERE category IS NULL OR statement IS NULL;

    ALTER TABLE settings ADD COLUMN contextual_review_enabled INTEGER NOT NULL DEFAULT 0
        CHECK (contextual_review_enabled IN (0, 1));
    ALTER TABLE settings ADD COLUMN activity_awareness_enabled INTEGER NOT NULL DEFAULT 0
        CHECK (activity_awareness_enabled IN (0, 1));
    ALTER TABLE settings ADD COLUMN notifications_enabled INTEGER NOT NULL DEFAULT 0
        CHECK (notifications_enabled IN (0, 1));
    ALTER TABLE settings ADD COLUMN intervention_cooldown_minutes INTEGER NOT NULL DEFAULT 90
        CHECK (intervention_cooldown_minutes >= 15 AND intervention_cooldown_minutes <= 1440);
    ALTER TABLE settings ADD COLUMN start_with_windows INTEGER NOT NULL DEFAULT 0
        CHECK (start_with_windows IN (0, 1));

    CREATE TABLE IF NOT EXISTS activity_events (
        id TEXT PRIMARY KEY NOT NULL,
        timestamp TEXT NOT NULL,
        event_type TEXT NOT NULL,
        application_name TEXT NULL,
        window_title TEXT NULL,
        duration_seconds INTEGER NULL CHECK (duration_seconds IS NULL OR duration_seconds >= 0)
    );
    CREATE INDEX IF NOT EXISTS idx_activity_events_timestamp ON activity_events(timestamp DESC);
    "#,
    ),
];

pub fn apply_migrations(connection: &mut Connection) -> Result<(), StorageError> {
    connection.execute_batch(
        "PRAGMA foreign_keys = ON; PRAGMA busy_timeout = 5000; PRAGMA journal_mode = WAL;",
    )?;
    connection.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_migrations (
            version INTEGER PRIMARY KEY NOT NULL,
            applied_at TEXT NOT NULL
        );",
    )?;

    for (version, sql) in MIGRATIONS {
        let applied = connection
            .query_row(
                "SELECT 1 FROM schema_migrations WHERE version = ?1 LIMIT 1",
                [version],
                |_| Ok(()),
            )
            .optional()?
            .is_some();

        if applied {
            continue;
        }

        let transaction = connection.transaction()?;
        transaction.execute_batch(sql)?;
        transaction.execute(
            "INSERT INTO schema_migrations (version, applied_at) VALUES (?1, ?2)",
            (version, Utc::now().to_rfc3339()),
        )?;
        transaction.commit()?;
    }

    Ok(())
}

#[cfg(test)]
pub fn current_schema_version(connection: &Connection) -> Result<i64, StorageError> {
    Ok(connection.query_row(
        "SELECT COALESCE(MAX(version), 0) FROM schema_migrations",
        [],
        |row| row.get(0),
    )?)
}
