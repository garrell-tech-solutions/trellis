use std::path::PathBuf;
use std::process::ExitCode;

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

async fn run_migrate(args: &[String]) -> ExitCode {
    let db_path = require_db_arg(args);
    let pool = match trellis_server::db::connect(&db_path).await {
        Ok(pool) => pool,
        Err(err) => {
            eprintln!("could not open database: {err}");
            return ExitCode::FAILURE;
        }
    };
    match trellis_server::db::run_migrations(&pool).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("migration failed: {err}");
            ExitCode::FAILURE
        }
    }
}

async fn bind_server(args: &[String]) -> Result<(tokio::net::TcpListener, axum::Router), String> {
    let db_path = require_db_arg(args);
    let addr = arg_value(args, "--addr").unwrap_or_else(|| "127.0.0.1:8080".to_string());
    let pool = trellis_server::db::connect(&db_path)
        .await
        .map_err(|err| format!("open database: {err}"))?;
    trellis_server::db::run_migrations(&pool)
        .await
        .map_err(|err| format!("run migrations: {err}"))?;
    let app = trellis_server::app::build_app(pool);
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
        _ => {
            eprintln!("usage: trellis <migrate|serve> --db <path> [--addr <host:port>]");
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
}
