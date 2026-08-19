use std::{
    fmt::{Display, Formatter},
    fs,
    path::Path,
};

use chrono::{DateTime, Utc};
use rusqlite::{params, types::Type, Connection, OptionalExtension, Row};

use crate::core::{
    action_execution::{ActionExecution, ExecutionStatus},
    candidate_action::{ActionStatus, CandidateAction},
    chat::{ChatMessage, ChatRole},
    goal::Goal,
    history::{HistoryEntry, HistoryKind},
    life_state::LifeState,
    memory::StoredMemory,
    outcome::ActionOutcome,
    profile::Profile,
    settings::LocalSettings,
};

use super::migrations;

#[derive(Debug)]
pub enum StorageError {
    Database(rusqlite::Error),
    Io(std::io::Error),
    Serialization(serde_json::Error),
    NotFound(String),
    Conflict(String),
}

impl Display for StorageError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Database(error) => write!(formatter, "Database error: {error}"),
            Self::Io(error) => write!(formatter, "Local storage error: {error}"),
            Self::Serialization(error) => {
                write!(formatter, "Stored data could not be read: {error}")
            }
            Self::NotFound(message) | Self::Conflict(message) => formatter.write_str(message),
        }
    }
}

impl std::error::Error for StorageError {}

impl From<rusqlite::Error> for StorageError {
    fn from(error: rusqlite::Error) -> Self {
        Self::Database(error)
    }
}

impl From<std::io::Error> for StorageError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<serde_json::Error> for StorageError {
    fn from(error: serde_json::Error) -> Self {
        Self::Serialization(error)
    }
}

#[derive(Debug, Clone)]
pub struct StoredDecision {
    pub id: String,
    pub timestamp: String,
    pub selected_action_id: Option<String>,
    pub score: f32,
    pub feasible: bool,
    pub reason: String,
    pub life_state_snapshot: String,
    pub ranking_snapshot: String,
}

pub struct SqliteRepository {
    connection: Connection,
}

impl SqliteRepository {
    pub fn open(path: &Path) -> Result<Self, StorageError> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let mut connection = Connection::open(path)?;
        migrations::apply_migrations(&mut connection)?;
        Ok(Self { connection })
    }

    #[cfg(test)]
    pub fn open_in_memory() -> Result<Self, StorageError> {
        let mut connection = Connection::open_in_memory()?;
        migrations::apply_migrations(&mut connection)?;
        Ok(Self { connection })
    }

    #[cfg(test)]
    pub fn schema_version(&self) -> Result<i64, StorageError> {
        migrations::current_schema_version(&self.connection)
    }

    pub fn get_profile(&self) -> Result<Option<Profile>, StorageError> {
        Ok(self
            .connection
            .query_row(
                "SELECT id, name, created_at, updated_at, onboarding_completed, default_available_minutes
                 FROM profile ORDER BY created_at ASC LIMIT 1",
                [],
                map_profile,
            )
            .optional()?)
    }

    pub fn update_profile_name(
        &mut self,
        id: &str,
        name: &str,
        updated_at: &str,
    ) -> Result<Profile, StorageError> {
        let changed = self.connection.execute(
            "UPDATE profile SET name = ?1, updated_at = ?2 WHERE id = ?3",
            params![name, updated_at, id],
        )?;
        require_changed(changed, "Profile not found.")?;
        self.get_profile()?
            .ok_or_else(|| StorageError::NotFound("Profile not found.".to_owned()))
    }

    pub fn get_settings(&self) -> Result<LocalSettings, StorageError> {
        let settings = self
            .connection
            .query_row(
                "SELECT start_week_day, default_available_minutes, ai_enabled, api_configuration_status,
                        contextual_review_enabled, activity_awareness_enabled, notifications_enabled,
                        intervention_cooldown_minutes, start_with_windows
                 FROM settings WHERE id = 1",
                [],
                |row| {
                    Ok(LocalSettings {
                        start_week_day: row.get(0)?,
                        default_available_minutes: row.get::<_, i64>(1)? as u32,
                        ai_enabled: row.get::<_, i64>(2)? != 0,
                        api_configuration_status: row.get(3)?,
                        contextual_review_enabled: row.get::<_, i64>(4)? != 0,
                        activity_awareness_enabled: row.get::<_, i64>(5)? != 0,
                        notifications_enabled: row.get::<_, i64>(6)? != 0,
                        intervention_cooldown_minutes: row.get::<_, i64>(7)? as u32,
                        start_with_windows: row.get::<_, i64>(8)? != 0,
                    })
                },
            )
            .optional()?;

        Ok(settings.unwrap_or_default())
    }

    pub fn upsert_settings(
        &mut self,
        settings: &LocalSettings,
    ) -> Result<LocalSettings, StorageError> {
        self.connection.execute(
            "INSERT INTO settings (
                id, start_week_day, default_available_minutes, ai_enabled, api_configuration_status,
                contextual_review_enabled, activity_awareness_enabled, notifications_enabled,
                intervention_cooldown_minutes, start_with_windows
             ) VALUES (1, ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
             ON CONFLICT(id) DO UPDATE SET
               start_week_day = excluded.start_week_day,
               default_available_minutes = excluded.default_available_minutes,
               ai_enabled = excluded.ai_enabled,
               api_configuration_status = excluded.api_configuration_status,
               contextual_review_enabled = excluded.contextual_review_enabled,
               activity_awareness_enabled = excluded.activity_awareness_enabled,
               notifications_enabled = excluded.notifications_enabled,
               intervention_cooldown_minutes = excluded.intervention_cooldown_minutes,
               start_with_windows = excluded.start_with_windows",
            params![
                settings.start_week_day,
                settings.default_available_minutes,
                bool_to_i64(settings.ai_enabled),
                settings.api_configuration_status,
                bool_to_i64(settings.contextual_review_enabled),
                bool_to_i64(settings.activity_awareness_enabled),
                bool_to_i64(settings.notifications_enabled),
                settings.intervention_cooldown_minutes,
                bool_to_i64(settings.start_with_windows),
            ],
        )?;
        self.connection.execute(
            "UPDATE profile SET default_available_minutes = ?1 WHERE onboarding_completed = 1",
            [settings.default_available_minutes],
        )?;
        self.get_settings()
    }

    pub fn complete_onboarding(
        &mut self,
        profile: &Profile,
        settings: &LocalSettings,
        goal: &Goal,
        life_state: &LifeState,
        actions: &[CandidateAction],
    ) -> Result<(), StorageError> {
        let transaction = self.connection.transaction()?;
        transaction.execute(
            "INSERT INTO profile (
                id, name, created_at, updated_at, onboarding_completed, default_available_minutes
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                profile.id,
                profile.name,
                profile.created_at,
                profile.updated_at,
                bool_to_i64(profile.onboarding_completed),
                profile.default_available_minutes,
            ],
        )?;
        transaction.execute(
            "INSERT INTO settings (
                id, start_week_day, default_available_minutes, ai_enabled, api_configuration_status,
                contextual_review_enabled, activity_awareness_enabled, notifications_enabled,
                intervention_cooldown_minutes, start_with_windows
             ) VALUES (1, ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                settings.start_week_day,
                settings.default_available_minutes,
                bool_to_i64(settings.ai_enabled),
                settings.api_configuration_status,
                bool_to_i64(settings.contextual_review_enabled),
                bool_to_i64(settings.activity_awareness_enabled),
                bool_to_i64(settings.notifications_enabled),
                settings.intervention_cooldown_minutes,
                bool_to_i64(settings.start_with_windows),
            ],
        )?;
        insert_life_state(&transaction, life_state)?;
        insert_goal(&transaction, goal)?;
        for action in actions {
            insert_action(&transaction, action)?;
        }
        transaction.commit()?;
        Ok(())
    }

    pub fn insert_life_state(&mut self, life_state: &LifeState) -> Result<(), StorageError> {
        insert_life_state(&self.connection, life_state)
    }

    pub fn latest_life_state(&self) -> Result<Option<LifeState>, StorageError> {
        Ok(self
            .connection
            .query_row(
                "SELECT id, timestamp, energy, focus, stress, sleep_hours, available_minutes, optional_note
                 FROM life_state_entries ORDER BY timestamp DESC, id DESC LIMIT 1",
                [],
                map_life_state,
            )
            .optional()?)
    }

    pub fn list_goals(&self) -> Result<Vec<Goal>, StorageError> {
        collect_rows(
            self.connection.prepare(
                "SELECT id, title, description, priority, active, created_at, updated_at, completed_at
                 FROM goals ORDER BY active DESC, priority DESC, created_at DESC",
            )?,
            [],
            map_goal,
        )
    }

    pub fn get_goal(&self, id: &str) -> Result<Option<Goal>, StorageError> {
        Ok(self
            .connection
            .query_row(
                "SELECT id, title, description, priority, active, created_at, updated_at, completed_at
                 FROM goals WHERE id = ?1",
                [id],
                map_goal,
            )
            .optional()?)
    }

    pub fn insert_goal(&mut self, goal: &Goal) -> Result<(), StorageError> {
        insert_goal(&self.connection, goal)
    }

    pub fn update_goal(&mut self, goal: &Goal) -> Result<Goal, StorageError> {
        let changed = self.connection.execute(
            "UPDATE goals SET title = ?1, description = ?2, priority = ?3, updated_at = ?4 WHERE id = ?5",
            params![goal.title, goal.description, goal.priority, goal.updated_at, goal.id],
        )?;
        require_changed(changed, "Goal not found.")?;
        self.get_goal(&goal.id)?
            .ok_or_else(|| StorageError::NotFound("Goal not found.".to_owned()))
    }

    pub fn set_goal_active(&mut self, id: &str, active: bool) -> Result<Goal, StorageError> {
        let changed = self.connection.execute(
            "UPDATE goals SET active = ?1, completed_at = CASE WHEN ?1 = 1 THEN NULL ELSE completed_at END,
             updated_at = ?2 WHERE id = ?3",
            params![bool_to_i64(active), chrono::Utc::now().to_rfc3339(), id],
        )?;
        require_changed(changed, "Goal not found.")?;
        self.get_goal(id)?
            .ok_or_else(|| StorageError::NotFound("Goal not found.".to_owned()))
    }

    pub fn complete_goal(&mut self, id: &str, completed_at: &str) -> Result<Goal, StorageError> {
        let changed = self.connection.execute(
            "UPDATE goals SET active = 0, completed_at = ?1, updated_at = ?1 WHERE id = ?2",
            params![completed_at, id],
        )?;
        require_changed(changed, "Goal not found.")?;
        self.get_goal(id)?
            .ok_or_else(|| StorageError::NotFound("Goal not found.".to_owned()))
    }

    pub fn delete_goal(&mut self, id: &str) -> Result<(), StorageError> {
        let changed = self
            .connection
            .execute("DELETE FROM goals WHERE id = ?1", [id])?;
        require_changed(changed, "Goal not found.")
    }

    pub fn list_actions(&self) -> Result<Vec<CandidateAction>, StorageError> {
        collect_rows(
            self.connection.prepare(
                "SELECT id, title, description, goal_id, impact, urgency, goal_alignment,
                        energy_required, duration_minutes, status, created_at, updated_at
                 FROM actions ORDER BY status ASC, created_at DESC",
            )?,
            [],
            map_action,
        )
    }

    pub fn get_action(&self, id: &str) -> Result<Option<CandidateAction>, StorageError> {
        Ok(self
            .connection
            .query_row(
                "SELECT id, title, description, goal_id, impact, urgency, goal_alignment,
                        energy_required, duration_minutes, status, created_at, updated_at
                 FROM actions WHERE id = ?1",
                [id],
                map_action,
            )
            .optional()?)
    }

    pub fn insert_action(&mut self, action: &CandidateAction) -> Result<(), StorageError> {
        insert_action(&self.connection, action)
    }

    pub fn update_action(
        &mut self,
        action: &CandidateAction,
    ) -> Result<CandidateAction, StorageError> {
        let changed = self.connection.execute(
            "UPDATE actions SET
                title = ?1,
                description = ?2,
                goal_id = ?3,
                impact = ?4,
                urgency = ?5,
                goal_alignment = ?6,
                energy_required = ?7,
                duration_minutes = ?8,
                updated_at = ?9
             WHERE id = ?10",
            params![
                action.title,
                action.description,
                action.goal_id,
                action.impact,
                action.urgency,
                action.goal_alignment,
                action.energy_required,
                action.duration_minutes,
                action.updated_at,
                action.id,
            ],
        )?;
        require_changed(changed, "Action not found.")?;
        self.get_action(&action.id)?
            .ok_or_else(|| StorageError::NotFound("Action not found.".to_owned()))
    }

    pub fn complete_action_item(&mut self, id: &str) -> Result<CandidateAction, StorageError> {
        let changed = self.connection.execute(
            "UPDATE actions SET status = 'completed', updated_at = ?1 WHERE id = ?2",
            params![chrono::Utc::now().to_rfc3339(), id],
        )?;
        require_changed(changed, "Action not found.")?;
        self.get_action(id)?
            .ok_or_else(|| StorageError::NotFound("Action not found.".to_owned()))
    }

    pub fn archive_action(&mut self, id: &str) -> Result<CandidateAction, StorageError> {
        let changed = self.connection.execute(
            "UPDATE actions SET status = 'archived', updated_at = ?1 WHERE id = ?2 AND status != 'in_progress'",
            params![chrono::Utc::now().to_rfc3339(), id],
        )?;
        require_changed(changed, "Action not found or currently in progress.")?;
        self.get_action(id)?
            .ok_or_else(|| StorageError::NotFound("Action not found.".to_owned()))
    }

    pub fn delete_action(&mut self, id: &str) -> Result<(), StorageError> {
        let changed = self
            .connection
            .execute("DELETE FROM actions WHERE id = ?1", [id])?;
        require_changed(changed, "Action not found.")
    }

    pub fn insert_decision(&mut self, decision: &StoredDecision) -> Result<(), StorageError> {
        self.connection.execute(
            "INSERT INTO decisions (
                id, timestamp, selected_action_id, score, feasible, reason,
                life_state_snapshot, ranking_snapshot
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                decision.id,
                decision.timestamp,
                decision.selected_action_id,
                decision.score,
                bool_to_i64(decision.feasible),
                decision.reason,
                decision.life_state_snapshot,
                decision.ranking_snapshot,
            ],
        )?;
        Ok(())
    }

    pub fn get_decision(&self, id: &str) -> Result<Option<StoredDecision>, StorageError> {
        Ok(self
            .connection
            .query_row(
                "SELECT id, timestamp, selected_action_id, score, feasible, reason,
                        life_state_snapshot, ranking_snapshot
                 FROM decisions WHERE id = ?1",
                [id],
                map_stored_decision,
            )
            .optional()?)
    }

    pub fn latest_decision(&self) -> Result<Option<StoredDecision>, StorageError> {
        Ok(self
            .connection
            .query_row(
                "SELECT id, timestamp, selected_action_id, score, feasible, reason,
                    life_state_snapshot, ranking_snapshot
             FROM decisions ORDER BY timestamp DESC, id DESC LIMIT 1",
                [],
                map_stored_decision,
            )
            .optional()?)
    }

    pub fn insert_execution(&mut self, execution: &ActionExecution) -> Result<(), StorageError> {
        let transaction = self.connection.transaction()?;
        match transaction.execute(
            "INSERT INTO executions (
                id, action_id, action_title, decision_id, decision_score, started_at,
                ended_at, status, energy_before
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, NULL, ?7, ?8)",
            params![
                execution.id,
                execution.action_id,
                execution.action_title,
                execution.decision_id,
                execution.decision_score,
                execution.started_at.to_rfc3339(),
                execution.status.as_str(),
                execution.energy_before,
            ],
        ) {
            Ok(_) => {
                let changed = transaction.execute(
                    "UPDATE actions SET status = 'in_progress', updated_at = ?1 WHERE id = ?2 AND status = 'available'",
                    params![execution.started_at.to_rfc3339(), execution.action_id],
                )?;
                require_changed(changed, "The action is no longer available.")?;
                transaction.commit()?;
                Ok(())
            }
            Err(rusqlite::Error::SqliteFailure(_, _)) => Err(StorageError::Conflict(
                "An action is already in progress.".to_owned(),
            )),
            Err(error) => Err(StorageError::Database(error)),
        }
    }

    pub fn get_active_execution(&self) -> Result<Option<ActionExecution>, StorageError> {
        Ok(self
            .connection
            .query_row(
                "SELECT id, action_id, action_title, decision_id, decision_score,
                        started_at, energy_before, status
                 FROM executions WHERE status = 'in_progress' ORDER BY started_at DESC LIMIT 1",
                [],
                map_execution,
            )
            .optional()?)
    }

    pub fn finish_execution_and_insert_outcome(
        &mut self,
        outcome: &ActionOutcome,
    ) -> Result<(), StorageError> {
        let transaction = self.connection.transaction()?;
        let final_status = if outcome.completed {
            "completed"
        } else {
            "abandoned"
        };
        let changed = transaction.execute(
            "UPDATE executions SET ended_at = ?1, status = ?2
             WHERE id = ?3 AND status = 'in_progress'",
            params![
                outcome.ended_at.to_rfc3339(),
                final_status,
                outcome.execution_id
            ],
        )?;
        if changed == 0 {
            return Err(StorageError::Conflict(
                "The active action is no longer in progress.".to_owned(),
            ));
        }

        transaction.execute(
            "INSERT INTO outcomes (
                id, execution_id, completed, abandoned, result_quality, difficulty,
                energy_before, energy_after, actual_duration_minutes, created_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                outcome.id,
                outcome.execution_id,
                bool_to_i64(outcome.completed),
                bool_to_i64(outcome.abandoned),
                outcome.result_quality,
                outcome.difficulty,
                outcome.energy_before,
                outcome.energy_after,
                outcome.actual_duration_minutes as i64,
                outcome.created_at.to_rfc3339(),
            ],
        )?;
        transaction.execute(
            "UPDATE actions SET status = ?1, updated_at = ?2 WHERE id = ?3",
            params![
                if outcome.completed {
                    "completed"
                } else {
                    "available"
                },
                outcome.ended_at.to_rfc3339(),
                outcome.action_id,
            ],
        )?;
        transaction.commit()?;
        Ok(())
    }

    pub fn recent_outcomes(&self, limit: usize) -> Result<Vec<ActionOutcome>, StorageError> {
        collect_rows(
            self.connection.prepare(
                "SELECT o.id, o.execution_id, e.action_id, e.action_title, e.decision_id,
                        e.decision_score, e.started_at, o.created_at,
                        o.actual_duration_minutes, o.completed, o.abandoned,
                        o.result_quality, o.energy_before, o.energy_after, o.difficulty
                 FROM outcomes o
                 INNER JOIN executions e ON e.id = o.execution_id
                 ORDER BY o.created_at DESC LIMIT ?1",
            )?,
            [limit as i64],
            map_outcome,
        )
    }

    pub fn insert_chat_message(&mut self, message: &ChatMessage) -> Result<(), StorageError> {
        self.connection.execute(
            "INSERT INTO chat_messages (id, role, content, timestamp) VALUES (?1, ?2, ?3, ?4)",
            params![
                message.id,
                message.role.as_str(),
                message.content,
                message.timestamp,
            ],
        )?;
        Ok(())
    }

    pub fn recent_chat_messages(&self, limit: usize) -> Result<Vec<ChatMessage>, StorageError> {
        let mut messages = collect_rows(
            self.connection.prepare(
                "SELECT id, role, content, timestamp FROM chat_messages
                 ORDER BY timestamp DESC, id DESC LIMIT ?1",
            )?,
            [limit as i64],
            map_chat_message,
        )?;
        messages.reverse();
        Ok(messages)
    }

    pub fn relevant_memories(&self, limit: usize) -> Result<Vec<StoredMemory>, StorageError> {
        collect_rows(
            self.connection.prepare(
                "SELECT id, COALESCE(category, type), COALESCE(statement, content), source,
                        confidence, importance, created_at, last_used_at, active
                 FROM memories WHERE active = 1
                 ORDER BY importance DESC, created_at DESC LIMIT ?1",
            )?,
            [limit as i64],
            map_memory,
        )
    }

    pub fn insert_memory_if_new(&mut self, memory: &StoredMemory) -> Result<bool, StorageError> {
        let exists = self
            .connection
            .query_row(
                "SELECT 1 FROM memories WHERE statement = ?1 AND active = 1 LIMIT 1",
                [&memory.statement],
                |_| Ok(()),
            )
            .optional()?
            .is_some();
        if exists {
            return Ok(false);
        }
        self.connection.execute(
            "INSERT INTO memories (
                id, type, content, importance, source, created_at, last_used_at,
                category, statement, confidence, active
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                memory.id,
                memory.category,
                memory.statement,
                memory.importance,
                memory.source,
                memory.created_at,
                memory.last_used_at,
                memory.category,
                memory.statement,
                memory.confidence,
                bool_to_i64(memory.active),
            ],
        )?;
        Ok(true)
    }

    pub fn history_entries(&self) -> Result<Vec<HistoryEntry>, StorageError> {
        let mut entries = Vec::new();

        entries.extend(collect_rows(
            self.connection.prepare(
                "SELECT id, timestamp, energy, focus, stress, sleep_hours, available_minutes
                 FROM life_state_entries ORDER BY timestamp DESC LIMIT 200",
            )?,
            [],
            |row| {
                let id: String = row.get(0)?;
                let timestamp: String = row.get(1)?;
                let energy: f64 = row.get(2)?;
                let focus: f64 = row.get(3)?;
                let stress: f64 = row.get(4)?;
                let sleep_hours: f64 = row.get(5)?;
                let available_minutes: i64 = row.get(6)?;
                Ok(HistoryEntry {
                    id,
                    kind: HistoryKind::LifeState,
                    timestamp,
                    title: "Life state check-in".to_owned(),
                    detail: format!(
                        "Energy {energy:.1}, focus {focus:.1}, stress {stress:.1}, sleep {sleep_hours:.1}h, {available_minutes} min available"
                    ),
                    status: None,
                })
            },
        )?);

        entries.extend(collect_rows(
            self.connection.prepare(
                "SELECT id, COALESCE(statement, content), created_at, confidence
                 FROM memories WHERE active = 1 ORDER BY created_at DESC LIMIT 200",
            )?,
            [],
            |row| {
                let id: String = row.get(0)?;
                let detail: String = row.get(1)?;
                let timestamp: String = row.get(2)?;
                let confidence: f64 = row.get(3)?;
                Ok(HistoryEntry {
                    id,
                    kind: HistoryKind::Memory,
                    timestamp,
                    title: "Memory learned".to_owned(),
                    detail,
                    status: Some(format!("confidence {:.0}%", confidence * 100.0)),
                })
            },
        )?);

        entries.extend(collect_rows(
            self.connection.prepare(
                "SELECT id, timestamp, selected_action_id, score, feasible, reason
                 FROM decisions ORDER BY timestamp DESC LIMIT 200",
            )?,
            [],
            |row| {
                let id: String = row.get(0)?;
                let timestamp: String = row.get(1)?;
                let selected_action_id: Option<String> = row.get(2)?;
                let score: f64 = row.get(3)?;
                let feasible = row.get::<_, i64>(4)? != 0;
                let reason: String = row.get(5)?;
                let title = selected_action_id
                    .map(|action_id| format!("Decision: {action_id}"))
                    .unwrap_or_else(|| "Decision: no feasible action".to_owned());
                Ok(HistoryEntry {
                    id,
                    kind: HistoryKind::Decision,
                    timestamp,
                    title,
                    detail: format!("Score {score:.2}. {reason}"),
                    status: Some(if feasible { "feasible" } else { "not_feasible" }.to_owned()),
                })
            },
        )?);

        entries.extend(collect_rows(
            self.connection.prepare(
                "SELECT id, action_title, started_at, status FROM executions
                 ORDER BY started_at DESC LIMIT 200",
            )?,
            [],
            |row| {
                let id: String = row.get(0)?;
                let action_title: String = row.get(1)?;
                let timestamp: String = row.get(2)?;
                let status: String = row.get(3)?;
                Ok(HistoryEntry {
                    id,
                    kind: HistoryKind::Execution,
                    timestamp,
                    title: format!("Started: {action_title}"),
                    detail: "Action execution started.".to_owned(),
                    status: Some(status),
                })
            },
        )?);

        entries.extend(collect_rows(
            self.connection.prepare(
                "SELECT o.id, o.created_at, e.action_title, o.completed, o.actual_duration_minutes
                 FROM outcomes o
                 INNER JOIN executions e ON e.id = o.execution_id
                 ORDER BY o.created_at DESC LIMIT 200",
            )?,
            [],
            |row| {
                let id: String = row.get(0)?;
                let timestamp: String = row.get(1)?;
                let action_title: String = row.get(2)?;
                let completed = row.get::<_, i64>(3)? != 0;
                let duration: i64 = row.get(4)?;
                let status = if completed { "completed" } else { "abandoned" };
                Ok(HistoryEntry {
                    id,
                    kind: HistoryKind::Outcome,
                    timestamp,
                    title: format!("{status}: {action_title}"),
                    detail: format!("Actual duration: {duration} min"),
                    status: Some(status.to_owned()),
                })
            },
        )?);

        entries.sort_by(|left, right| right.timestamp.cmp(&left.timestamp));
        Ok(entries)
    }
}

fn insert_life_state(connection: &Connection, life_state: &LifeState) -> Result<(), StorageError> {
    connection.execute(
        "INSERT INTO life_state_entries (
            id, timestamp, energy, focus, stress, sleep_hours, available_minutes, optional_note
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            life_state.id,
            life_state.timestamp,
            life_state.energy,
            life_state.focus,
            life_state.stress,
            life_state.sleep_hours,
            life_state.available_minutes,
            life_state.optional_note,
        ],
    )?;
    Ok(())
}

fn insert_goal(connection: &Connection, goal: &Goal) -> Result<(), StorageError> {
    connection.execute(
        "INSERT INTO goals (id, title, description, priority, active, created_at, updated_at, completed_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            goal.id,
            goal.title,
            goal.description,
            goal.priority,
            bool_to_i64(goal.active),
            goal.created_at,
            goal.updated_at,
            goal.completed_at,
        ],
    )?;
    Ok(())
}

fn insert_action(connection: &Connection, action: &CandidateAction) -> Result<(), StorageError> {
    connection.execute(
        "INSERT INTO actions (
            id, title, description, goal_id, impact, urgency, goal_alignment,
            energy_required, duration_minutes, status, created_at, updated_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
        params![
            action.id,
            action.title,
            action.description,
            action.goal_id,
            action.impact,
            action.urgency,
            action.goal_alignment,
            action.energy_required,
            action.duration_minutes,
            action.status.as_str(),
            action.created_at,
            action.updated_at,
        ],
    )?;
    Ok(())
}

fn map_profile(row: &Row<'_>) -> rusqlite::Result<Profile> {
    Ok(Profile {
        id: row.get(0)?,
        name: row.get(1)?,
        created_at: row.get(2)?,
        updated_at: row.get(3)?,
        onboarding_completed: row.get::<_, i64>(4)? != 0,
        default_available_minutes: row.get::<_, i64>(5)? as u32,
    })
}

fn map_life_state(row: &Row<'_>) -> rusqlite::Result<LifeState> {
    Ok(LifeState {
        id: row.get(0)?,
        timestamp: row.get(1)?,
        energy: row.get::<_, f64>(2)? as f32,
        focus: row.get::<_, f64>(3)? as f32,
        stress: row.get::<_, f64>(4)? as f32,
        sleep_hours: row.get::<_, f64>(5)? as f32,
        available_minutes: row.get::<_, i64>(6)? as u32,
        optional_note: row.get(7)?,
    })
}

fn map_goal(row: &Row<'_>) -> rusqlite::Result<Goal> {
    Ok(Goal {
        id: row.get(0)?,
        title: row.get(1)?,
        description: row.get(2)?,
        priority: row.get::<_, f64>(3)? as f32,
        active: row.get::<_, i64>(4)? != 0,
        created_at: row.get(5)?,
        updated_at: row.get(6)?,
        completed_at: row.get(7)?,
    })
}

fn map_action(row: &Row<'_>) -> rusqlite::Result<CandidateAction> {
    let status: String = row.get(9)?;
    Ok(CandidateAction {
        id: row.get(0)?,
        title: row.get(1)?,
        description: row.get(2)?,
        goal_id: row.get(3)?,
        impact: row.get::<_, f64>(4)? as f32,
        urgency: row.get::<_, f64>(5)? as f32,
        goal_alignment: row.get::<_, f64>(6)? as f32,
        energy_required: row.get::<_, f64>(7)? as f32,
        duration_minutes: row.get::<_, i64>(8)? as u32,
        status: ActionStatus::from_db(&status).map_err(|message| data_error(9, message))?,
        created_at: row.get(10)?,
        updated_at: row.get(11)?,
    })
}

fn map_chat_message(row: &Row<'_>) -> rusqlite::Result<ChatMessage> {
    let role: String = row.get(1)?;
    Ok(ChatMessage {
        id: row.get(0)?,
        role: ChatRole::from_db(&role).map_err(|message| data_error(1, message))?,
        content: row.get(2)?,
        timestamp: row.get(3)?,
    })
}

fn map_memory(row: &Row<'_>) -> rusqlite::Result<StoredMemory> {
    Ok(StoredMemory {
        id: row.get(0)?,
        category: row.get(1)?,
        statement: row.get(2)?,
        source: row.get(3)?,
        confidence: row.get::<_, f64>(4)? as f32,
        importance: row.get::<_, f64>(5)? as f32,
        created_at: row.get(6)?,
        last_used_at: row.get(7)?,
        active: row.get::<_, i64>(8)? != 0,
    })
}

fn map_stored_decision(row: &Row<'_>) -> rusqlite::Result<StoredDecision> {
    Ok(StoredDecision {
        id: row.get(0)?,
        timestamp: row.get(1)?,
        selected_action_id: row.get(2)?,
        score: row.get::<_, f64>(3)? as f32,
        feasible: row.get::<_, i64>(4)? != 0,
        reason: row.get(5)?,
        life_state_snapshot: row.get(6)?,
        ranking_snapshot: row.get(7)?,
    })
}

fn map_execution(row: &Row<'_>) -> rusqlite::Result<ActionExecution> {
    let status: String = row.get(7)?;
    Ok(ActionExecution {
        id: row.get(0)?,
        action_id: row.get(1)?,
        action_title: row.get(2)?,
        decision_id: row.get(3)?,
        decision_score: row.get::<_, f64>(4)? as f32,
        started_at: parse_timestamp(row, 5)?,
        energy_before: row.get::<_, f64>(6)? as f32,
        status: ExecutionStatus::from_db(&status).map_err(|message| data_error(7, message))?,
    })
}

fn map_outcome(row: &Row<'_>) -> rusqlite::Result<ActionOutcome> {
    Ok(ActionOutcome {
        id: row.get(0)?,
        execution_id: row.get(1)?,
        action_id: row.get(2)?,
        action_title: row.get(3)?,
        decision_id: row.get(4)?,
        decision_score: row.get::<_, f64>(5)? as f32,
        recommended: true,
        accepted: true,
        started_at: parse_timestamp(row, 6)?,
        ended_at: parse_timestamp(row, 7)?,
        created_at: parse_timestamp(row, 7)?,
        actual_duration_minutes: row.get::<_, i64>(8)? as u64,
        completed: row.get::<_, i64>(9)? != 0,
        abandoned: row.get::<_, i64>(10)? != 0,
        result_quality: row.get::<_, Option<f64>>(11)?.map(|value| value as f32),
        energy_before: row.get::<_, f64>(12)? as f32,
        energy_after: row.get::<_, Option<f64>>(13)?.map(|value| value as f32),
        difficulty: row.get::<_, Option<f64>>(14)?.map(|value| value as f32),
    })
}

fn parse_timestamp(row: &Row<'_>, index: usize) -> rusqlite::Result<DateTime<Utc>> {
    let raw: String = row.get(index)?;
    DateTime::parse_from_rfc3339(&raw)
        .map(|timestamp| timestamp.with_timezone(&Utc))
        .map_err(|error| data_error(index, format!("Invalid stored timestamp '{raw}': {error}")))
}

fn data_error(index: usize, message: String) -> rusqlite::Error {
    rusqlite::Error::FromSqlConversionFailure(
        index,
        Type::Text,
        Box::new(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            message,
        )),
    )
}

fn bool_to_i64(value: bool) -> i64 {
    i64::from(value)
}

fn require_changed(changed: usize, message: &str) -> Result<(), StorageError> {
    if changed == 0 {
        Err(StorageError::NotFound(message.to_owned()))
    } else {
        Ok(())
    }
}

fn collect_rows<T, P, F>(
    mut statement: rusqlite::Statement<'_>,
    parameters: P,
    mapper: F,
) -> Result<Vec<T>, StorageError>
where
    P: rusqlite::Params,
    F: FnMut(&Row<'_>) -> rusqlite::Result<T>,
{
    let rows = statement.query_map(parameters, mapper)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
}

#[cfg(test)]
mod tests {
    use super::SqliteRepository;

    #[test]
    fn database_initialization_applies_the_current_schema_migration() {
        let repository = SqliteRepository::open_in_memory().expect("database should initialize");

        assert_eq!(
            repository
                .schema_version()
                .expect("schema version should load"),
            3
        );
        assert_eq!(
            repository
                .get_settings()
                .expect("default settings should load")
                .start_week_day,
            "monday"
        );
    }
}
