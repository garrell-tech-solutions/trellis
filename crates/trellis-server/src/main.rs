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

#[tokio::main]
async fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(String::as_str) {
        Some("migrate") => {
            let db_path = require_db_arg(&args[2..]);
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
        Some("serve") => {
            let db_path = require_db_arg(&args[2..]);
            let addr =
                arg_value(&args[2..], "--addr").unwrap_or_else(|| "127.0.0.1:8080".to_string());
            let pool = trellis_server::db::connect(&db_path)
                .await
                .expect("open database");
            trellis_server::db::run_migrations(&pool)
                .await
                .expect("run migrations");
            let app = trellis_server::app::build_app(pool);
            let listener = tokio::net::TcpListener::bind(&addr)
                .await
                .expect("bind address");
            axum::serve(listener, app).await.expect("serve");
            ExitCode::SUCCESS
        }
        _ => {
            eprintln!("usage: trellis <migrate|serve> --db <path> [--addr <host:port>]");
            ExitCode::from(2)
        }
    }
}
