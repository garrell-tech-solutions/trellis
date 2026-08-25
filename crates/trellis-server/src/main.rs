use std::path::{Path, PathBuf};
use std::process::ExitCode;
use trellis_server::platform::clock::Clock;

/// What `trellis --version` says when the binary was not built by ci.yml's
/// `release` job -- a developer's `cargo run`, or the pull-request build
/// ops/preview.sh installs. Neither is a release, and both have to say so
/// rather than borrow an identity they do not have.
const UNRELEASED: &str = "unreleased";
const UNKNOWN_COMMIT: &str = "unknown-commit";

/// The one line `trellis --version` prints: the release tag this binary was
/// published under and the commit it was built from.
///
/// Both are read with `option_env!`, not from `CARGO_PKG_VERSION`. The three
/// workspace crates are all `0.1.0` and nothing consumes them; the tag is
/// derived from the CI run that publishes the release (see the `release` job
/// in .github/workflows/ci.yml), so there is no version field for a human to
/// forget to bump and no file for it to go stale in. Cargo tracks
/// `option_env!` reads as build dependencies, so changing either value
/// rebuilds this crate rather than silently keeping a cached binary that
/// names the wrong release.
///
/// The arguments are parameters rather than `option_env!` calls inside the
/// body so that both the released and the unreleased shape are reachable from
/// a test; a build can only ever exercise one of them.
///
/// The format is load-bearing, not decoration. ops/update.sh compares this
/// string against `trellis <tag> (<commit>)` built from the release it just
/// downloaded, and refuses to install a binary that does not match -- which
/// is what makes "update.sh installed what it said it did" checkable instead
/// of merely claimed.
fn version_line(release: Option<&str>, commit: Option<&str>) -> String {
    format!(
        "trellis {} ({})",
        release.unwrap_or(UNRELEASED),
        commit.unwrap_or(UNKNOWN_COMMIT)
    )
}

fn run_version() -> ExitCode {
    println!(
        "{}",
        version_line(
            option_env!("TRELLIS_RELEASE"),
            option_env!("TRELLIS_COMMIT")
        )
    );
    ExitCode::SUCCESS
}

fn arg_value(args: &[String], flag: &str) -> Option<String> {
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        if arg == flag {
            return iter.next().cloned();
        }
    }
    None
}

fn require_db_arg(args: &[String]) -> PathBuf {
    match arg_value(args, "--db") {
        Some(path) => PathBuf::from(path),
        None => {
            eprintln!("missing required --db <path>");
            std::process::exit(2);
        }
    }
}

/// Opens the database at `db_path` and brings it up to date, the one setup
/// sequence both `migrate` and `serve` run before anything else.
async fn connect_and_migrate(db_path: &Path) -> Result<sqlx::SqlitePool, String> {
    let pool = trellis_server::platform::db::connect(db_path)
        .await
        .map_err(|err| format!("open database: {err}"))?;
    trellis_server::platform::db::run_migrations(&pool)
        .await
        .map_err(|err| format!("run migrations: {err}"))?;
    Ok(pool)
}

async fn run_migrate(args: &[String]) -> ExitCode {
    let db_path = require_db_arg(args);
    match connect_and_migrate(&db_path).await {
        Ok(_) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("{err}");
            ExitCode::FAILURE
        }
    }
}

/// Parses `--now`'s `RFC3339` value to epoch milliseconds. `None` when the
/// flag was not given -- the server then runs on the real clock, unpinned.
fn parse_now_arg(args: &[String]) -> Result<Option<i64>, String> {
    match arg_value(args, "--now") {
        None => Ok(None),
        Some(value) => value
            .parse::<jiff::Timestamp>()
            .map(|ts| Some(ts.as_millisecond()))
            .map_err(|err| format!("invalid --now {value:?}: {err}")),
    }
}

/// The clock this server will run on: pinned when `--now` was given, the real
/// one otherwise. Chosen here, in the one place that reads the command line,
/// and handed to `build_app` — nothing downstream decides what time it is.
fn clock_for(pinned_now_ms: Option<i64>) -> Clock {
    match pinned_now_ms {
        Some(pinned_now_ms) => Clock::pinned_at(pinned_now_ms),
        None => Clock::system(),
    }
}

async fn bind_server(args: &[String]) -> Result<(tokio::net::TcpListener, axum::Router), String> {
    let db_path = require_db_arg(args);
    let addr = arg_value(args, "--addr").unwrap_or_else(|| "127.0.0.1:8080".to_string());
    let clock = clock_for(parse_now_arg(args)?);
    let pool = connect_and_migrate(&db_path).await?;
    let app = trellis_server::platform::app::build_app(pool, clock);
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .map_err(|err| format!("bind {addr}: {err}"))?;
    Ok((listener, app))
}

async fn run_serve(args: &[String]) -> ExitCode {
    match bind_server(args).await {
        Ok((listener, app)) => {
            axum::serve(listener, app).await.expect("serve");
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("{err}");
            ExitCode::FAILURE
        }
    }
}

async fn dispatch(args: &[String]) -> ExitCode {
    match args.get(1).map(String::as_str) {
        Some("migrate") => run_migrate(&args[2..]).await,
        Some("serve") => run_serve(&args[2..]).await,
        Some("version") | Some("--version") => run_version(),
        _ => {
            eprintln!(
                "usage: trellis <migrate|serve> --db <path> [--addr <host:port>] [--now <RFC3339>]\n       trellis --version"
            );
            ExitCode::from(2)
        }
    }
}

#[tokio::main]
async fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    dispatch(&args).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_line_names_the_release_and_the_commit_it_was_built_from() {
        assert_eq!(
            version_line(Some("v2026.08.24.1042"), Some("c452751")),
            "trellis v2026.08.24.1042 (c452751)"
        );
    }

    #[test]
    fn version_line_refuses_to_claim_a_release_it_was_not_built_for() {
        assert_eq!(
            version_line(None, None),
            format!("trellis {UNRELEASED} ({UNKNOWN_COMMIT})")
        );
    }

    #[test]
    fn version_line_reports_a_known_commit_even_without_a_release() {
        assert_eq!(
            version_line(None, Some("c452751")),
            format!("trellis {UNRELEASED} (c452751)")
        );
    }

    #[test]
    fn run_version_succeeds() {
        assert_eq!(run_version(), ExitCode::SUCCESS);
    }

    #[test]
    fn arg_value_returns_the_value_following_a_present_flag() {
        let args = vec!["--db".to_string(), "captures.sqlite".to_string()];
        assert_eq!(
            arg_value(&args, "--db"),
            Some("captures.sqlite".to_string())
        );
    }

    #[test]
    fn arg_value_returns_none_for_an_absent_flag() {
        let args = vec!["--addr".to_string(), "127.0.0.1:9090".to_string()];
        assert_eq!(arg_value(&args, "--db"), None);
    }

    #[test]
    fn require_db_arg_parses_the_db_path_when_present() {
        let args = vec!["--db".to_string(), "captures.sqlite".to_string()];
        assert_eq!(require_db_arg(&args), PathBuf::from("captures.sqlite"));
    }

    #[test]
    fn parse_now_arg_returns_none_when_the_flag_is_absent() {
        let args = vec!["--db".to_string(), "captures.sqlite".to_string()];
        assert_eq!(parse_now_arg(&args), Ok(None));
    }

    #[test]
    fn parse_now_arg_parses_an_rfc3339_value_to_epoch_milliseconds() {
        let args = vec!["--now".to_string(), "2026-07-24T09:00:00Z".to_string()];
        assert_eq!(parse_now_arg(&args), Ok(Some(1784883600000)));
    }

    #[test]
    fn parse_now_arg_rejects_an_unparseable_value() {
        let args = vec!["--now".to_string(), "not-a-timestamp".to_string()];
        assert!(parse_now_arg(&args).is_err());
    }

    #[test]
    fn clock_for_a_pinned_instant_reads_that_instant() {
        let pinned = 1784883600000;

        let got = clock_for(Some(pinned)).now_ms();

        assert!(
            (pinned..pinned + 1_000).contains(&got),
            "expected a clock reading roughly {pinned}, got {got}"
        );
    }

    #[test]
    fn clock_for_no_pin_reads_the_real_clock() {
        let real = jiff::Timestamp::now().as_millisecond();

        let got = clock_for(None).now_ms();

        assert!(
            (got - real).abs() < 1_000,
            "expected a clock reading roughly {real}, got {got}"
        );
    }

    #[tokio::test]
    async fn run_migrate_succeeds_against_a_fresh_database() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let args = vec!["--db".to_string(), db_path.to_string_lossy().to_string()];
        assert_eq!(run_migrate(&args).await, ExitCode::SUCCESS);
    }

    #[tokio::test]
    async fn run_migrate_fails_when_the_database_cannot_be_opened() {
        let args = vec![
            "--db".to_string(),
            "/nonexistent-dir/captures.sqlite".to_string(),
        ];
        assert_eq!(run_migrate(&args).await, ExitCode::FAILURE);
    }

    #[tokio::test]
    async fn dispatch_runs_migrate_for_the_migrate_subcommand() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let args = vec![
            "trellis".to_string(),
            "migrate".to_string(),
            "--db".to_string(),
            db_path.to_string_lossy().to_string(),
        ];
        assert_eq!(dispatch(&args).await, ExitCode::SUCCESS);
    }

    #[tokio::test]
    async fn dispatch_binds_and_serves_for_the_serve_subcommand() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let args = vec![
            "trellis".to_string(),
            "serve".to_string(),
            "--db".to_string(),
            db_path.to_string_lossy().to_string(),
            "--addr".to_string(),
            "127.0.0.1:0".to_string(),
        ];
        let handle = tokio::spawn(async move { dispatch(&args).await });
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        assert!(!handle.is_finished(), "serve returned before being stopped");
        handle.abort();
    }

    #[tokio::test]
    async fn dispatch_reports_usage_for_an_unknown_subcommand() {
        let args = vec!["trellis".to_string(), "bogus".to_string()];
        assert_eq!(dispatch(&args).await, ExitCode::from(2));
    }

    #[tokio::test]
    async fn dispatch_reports_the_version_for_the_version_flag() {
        let args = vec!["trellis".to_string(), "--version".to_string()];
        assert_eq!(dispatch(&args).await, ExitCode::SUCCESS);
    }

    #[tokio::test]
    async fn dispatch_reports_the_version_for_the_version_subcommand() {
        let args = vec!["trellis".to_string(), "version".to_string()];
        assert_eq!(dispatch(&args).await, ExitCode::SUCCESS);
    }

    #[tokio::test]
    async fn bind_server_binds_to_an_ephemeral_port_on_a_fresh_database() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let args = vec![
            "--db".to_string(),
            db_path.to_string_lossy().to_string(),
            "--addr".to_string(),
            "127.0.0.1:0".to_string(),
        ];
        let (listener, _app) = bind_server(&args)
            .await
            .expect("bind_server should succeed");
        assert_eq!(listener.local_addr().unwrap().ip().to_string(), "127.0.0.1");
    }

    #[tokio::test]
    async fn bind_server_fails_when_the_database_cannot_be_opened() {
        let args = vec![
            "--db".to_string(),
            "/nonexistent-dir/captures.sqlite".to_string(),
            "--addr".to_string(),
            "127.0.0.1:0".to_string(),
        ];
        assert!(bind_server(&args).await.is_err());
    }

    #[tokio::test]
    async fn bind_server_succeeds_with_a_now_flag_present() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let args = vec![
            "--db".to_string(),
            db_path.to_string_lossy().to_string(),
            "--addr".to_string(),
            "127.0.0.1:0".to_string(),
            "--now".to_string(),
            "2026-07-24T09:00:00Z".to_string(),
        ];
        assert!(bind_server(&args).await.is_ok());
    }

    #[tokio::test]
    async fn bind_server_fails_when_now_is_not_a_valid_timestamp() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let args = vec![
            "--db".to_string(),
            db_path.to_string_lossy().to_string(),
            "--addr".to_string(),
            "127.0.0.1:0".to_string(),
            "--now".to_string(),
            "not-a-timestamp".to_string(),
        ];
        assert!(bind_server(&args).await.is_err());
    }
}
