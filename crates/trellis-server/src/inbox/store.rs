//! What the page reads: the untriaged queue, and the tasks triage has
//! produced.
//!
//! Both are the inbox's queries even though neither table is the inbox's
//! alone — a capability owns the SQL it issues. Which columns these select
//! is this module's business; what a page does with them is not
//! (`T-templates-take-view-models`, and see [`super::view`]).

use sqlx::SqlitePool;

/// A row of [`list_untriaged`].
#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
pub struct UntriagedCapture {
    pub id: i64,
    pub raw_text: String,
}

/// Untriaged captures, newest first — the inbox's contents.
pub async fn list_untriaged(pool: &SqlitePool) -> Result<Vec<UntriagedCapture>, sqlx::Error> {
    sqlx::query_as("SELECT id, raw_text FROM captures WHERE triaged_at IS NULL ORDER BY id DESC")
        .fetch_all(pool)
        .await
}

/// A row of [`list_tasks`]: a task, alongside the text of the capture it was
/// triaged from and the name of the life area it was tagged with — the task
/// list's own rows have no text or life-area name of their own to show, so
/// both joins are this query's business, not the page's. `life_area_name` is
/// a `LEFT JOIN`: an archived life area still resolves (it is retired, not
/// deleted), and this query does not filter on that state at all.
#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
pub struct TaskWithCaptureText {
    pub kind: String,
    pub raw_text: String,
    pub life_area_name: Option<String>,
}

/// Every task, newest first.
pub async fn list_tasks(pool: &SqlitePool) -> Result<Vec<TaskWithCaptureText>, sqlx::Error> {
    sqlx::query_as(
        "SELECT tasks.kind, captures.raw_text, life_areas.name AS life_area_name FROM tasks \
         JOIN captures ON captures.id = tasks.capture_id \
         LEFT JOIN life_areas ON life_areas.id = tasks.life_area_id \
         ORDER BY tasks.id DESC",
    )
    .fetch_all(pool)
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::test_support::test_pool;
    use crate::triage::store::{insert_task, mark_triaged};
    use proptest::prelude::*;
    use scheduler_core::task::TaskKind;

    /// Setting up "a capture exists" by calling the capture domain's own
    /// writer rather than retyping its `INSERT` here: a fixture that spells
    /// out another module's SQL is a second copy of that schema.
    async fn given_a_capture(pool: &SqlitePool, raw_text: &str) -> i64 {
        crate::capture::store::insert(pool, raw_text, "web", 0)
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn list_untriaged_reports_each_captures_id() {
        let (_dir, pool) = test_pool().await;
        let id = given_a_capture(&pool, "buy milk").await;

        let captures = list_untriaged(&pool).await.unwrap();

        assert_eq!(captures.len(), 1);
        assert_eq!(captures[0].id, id);
    }

    #[tokio::test]
    async fn list_untriaged_is_empty_against_a_fresh_database() {
        let (_dir, pool) = test_pool().await;

        assert_eq!(list_untriaged(&pool).await.unwrap(), Vec::new());
    }

    #[tokio::test]
    async fn list_untriaged_lists_captures_newest_first() {
        let (_dir, pool) = test_pool().await;
        given_a_capture(&pool, "call the dentist").await;
        given_a_capture(&pool, "buy milk").await;

        let captures = list_untriaged(&pool).await.unwrap();

        assert_eq!(
            captures
                .iter()
                .map(|c| c.raw_text.as_str())
                .collect::<Vec<_>>(),
            vec!["buy milk", "call the dentist"]
        );
    }

    #[tokio::test]
    async fn list_untriaged_excludes_a_triaged_capture() {
        let (_dir, pool) = test_pool().await;
        given_a_capture(&pool, "buy milk").await;
        let triaged = given_a_capture(&pool, "call the dentist").await;
        mark_triaged(&pool, triaged, 9999).await.unwrap();

        let captures = list_untriaged(&pool).await.unwrap();

        assert_eq!(captures.len(), 1);
        assert_eq!(captures[0].raw_text, "buy milk");
    }

    #[tokio::test]
    async fn list_tasks_is_empty_against_a_fresh_database() {
        let (_dir, pool) = test_pool().await;

        assert_eq!(list_tasks(&pool).await.unwrap(), Vec::new());
    }

    #[tokio::test]
    async fn list_tasks_reports_each_tasks_kind_and_the_text_of_the_capture_it_came_from() {
        let (_dir, pool) = test_pool().await;
        let capture_id = given_a_capture(&pool, "buy milk").await;

        insert_task(&pool, capture_id, &TaskKind::Pool, None, 7)
            .await
            .unwrap();

        assert_eq!(
            list_tasks(&pool).await.unwrap(),
            vec![TaskWithCaptureText {
                kind: "pool".to_string(),
                raw_text: "buy milk".to_string(),
                life_area_name: None,
            }]
        );
    }

    #[tokio::test]
    async fn list_tasks_reports_the_name_of_the_life_area_a_task_was_tagged_with() {
        let (_dir, pool) = test_pool().await;
        let capture_id = given_a_capture(&pool, "buy milk").await;
        let learning_id = crate::life_areas::store::find_by_name(&pool, "Learning")
            .await
            .unwrap()
            .unwrap()
            .id;

        insert_task(&pool, capture_id, &TaskKind::Pool, Some(learning_id), 7)
            .await
            .unwrap();

        let tasks = list_tasks(&pool).await.unwrap();
        assert_eq!(tasks[0].life_area_name.as_deref(), Some("Learning"));
    }

    #[tokio::test]
    async fn list_tasks_still_resolves_the_name_of_an_archived_life_area() {
        let (_dir, pool) = test_pool().await;
        let capture_id = given_a_capture(&pool, "buy milk").await;
        let learning_id = crate::life_areas::store::find_by_name(&pool, "Learning")
            .await
            .unwrap()
            .unwrap()
            .id;
        insert_task(&pool, capture_id, &TaskKind::Pool, Some(learning_id), 7)
            .await
            .unwrap();
        crate::life_areas::store::archive(&pool, learning_id, 1_000)
            .await
            .unwrap();

        let tasks = list_tasks(&pool).await.unwrap();

        assert_eq!(tasks[0].life_area_name.as_deref(), Some("Learning"));
    }

    #[tokio::test]
    async fn list_tasks_lists_tasks_newest_first() {
        let (_dir, pool) = test_pool().await;
        let first_capture = given_a_capture(&pool, "buy milk").await;
        let second_capture = given_a_capture(&pool, "call the dentist").await;

        insert_task(&pool, first_capture, &TaskKind::Pool, None, 1)
            .await
            .unwrap();
        insert_task(&pool, second_capture, &TaskKind::Pool, None, 2)
            .await
            .unwrap();

        let tasks = list_tasks(&pool).await.unwrap();
        assert_eq!(
            tasks
                .iter()
                .map(|t| t.raw_text.as_str())
                .collect::<Vec<_>>(),
            vec!["call the dentist", "buy milk"]
        );
    }

    proptest! {
        #![proptest_config(ProptestConfig { cases: 32, ..ProptestConfig::default() })]

        /// The listing is a *partition*: exactly the untriaged captures, in
        /// exactly newest-first order. The example tests above sample two
        /// captures and one triaged one; this pins both halves — the `WHERE`
        /// and the `ORDER BY` — against any queue and any triaged subset of
        /// it, which is the shape the inbox actually meets.
        #[test]
        #[ignore]
        fn list_untriaged_returns_exactly_the_untriaged_captures_newest_first(
            queue in prop::collection::vec((".{0,40}", any::<bool>()), 0..12),
        ) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            let (expected, listed) = rt.block_on(async {
                let (_dir, pool) = test_pool().await;

                let mut expected: Vec<String> = Vec::new();
                for (raw_text, triaged) in &queue {
                    let id = given_a_capture(&pool, raw_text).await;
                    if *triaged {
                        mark_triaged(&pool, id, 9999).await.unwrap();
                    } else {
                        expected.push(raw_text.clone());
                    }
                }
                expected.reverse();

                let listed: Vec<String> = list_untriaged(&pool)
                    .await
                    .unwrap()
                    .into_iter()
                    .map(|capture| capture.raw_text)
                    .collect();
                (expected, listed)
            });

            prop_assert_eq!(expected, listed);
        }
    }
}
