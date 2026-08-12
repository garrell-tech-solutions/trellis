use super::*;
use std::os::unix::fs::PermissionsExt;
use std::process::Command;

const MUSL_TARGET: &str = "x86_64-unknown-linux-musl";

static GIVEN_WORKSPACE_CHECKED_OUT: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^the workspace is checked out$").unwrap());
static WHEN_RELEASE_BUILT: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^the release binary is built for the musl target$").unwrap());
static THEN_EXACTLY_ONE_BINARY: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^exactly one release binary is produced$").unwrap());
static THEN_NO_DYNAMIC_DEPS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^the binary reports no dynamic executable dependencies$").unwrap()
});
static WHEN_DEP_TREE_LISTED: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^the dependency tree for the scheduler-core crate is listed$").unwrap()
});
static THEN_DEP_TREE_EXCLUDES: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"^the dependency tree contains zero occurrences of "<([A-Za-z0-9_]+)>"$"#).unwrap()
});

pub fn dispatch(
    world: &mut World,
    text: &str,
    example: &BTreeMap<String, String>,
) -> Option<Result<(), String>> {
    dispatch_release_binary(world, text).or_else(|| dispatch_dependency_tree(world, text, example))
}

fn dispatch_release_binary(world: &mut World, text: &str) -> Option<Result<(), String>> {
    if GIVEN_WORKSPACE_CHECKED_OUT.is_match(text) {
        return Some(given_workspace_checked_out(&workspace_root()));
    }
    if WHEN_RELEASE_BUILT.is_match(text) {
        return Some(when_release_binary_built_for_musl(world));
    }
    if THEN_EXACTLY_ONE_BINARY.is_match(text) {
        return Some(then_exactly_one_release_binary(world));
    }
    if THEN_NO_DYNAMIC_DEPS.is_match(text) {
        return Some(then_binary_has_no_dynamic_dependencies(world));
    }
    None
}

fn dispatch_dependency_tree(
    world: &mut World,
    text: &str,
    example: &BTreeMap<String, String>,
) -> Option<Result<(), String>> {
    if WHEN_DEP_TREE_LISTED.is_match(text) {
        return Some(when_scheduler_core_dependency_tree_listed(world));
    }
    if let Some(caps) = THEN_DEP_TREE_EXCLUDES.captures(text) {
        let forbidden = match example_value(example, &caps[1]) {
            Ok(v) => v,
            Err(e) => return Some(Err(e)),
        };
        return Some(then_dependency_tree_excludes(world, forbidden));
    }
    None
}

pub fn given_workspace_checked_out(root: &Path) -> Result<(), String> {
    if root.join("Cargo.toml").is_file() {
        Ok(())
    } else {
        Err(format!("no workspace Cargo.toml under {}", root.display()))
    }
}

fn is_executable(path: &Path) -> bool {
    let Ok(metadata) = path.metadata() else {
        return false;
    };
    metadata.permissions().mode() & 0o111 != 0
}

fn executable_files_in(dir: &Path) -> Result<Vec<PathBuf>, String> {
    let mut files = Vec::new();
    for entry in std::fs::read_dir(dir).map_err(|e| format!("read {}: {e}", dir.display()))? {
        let entry = entry.map_err(|e| format!("read dir entry: {e}"))?;
        let path = entry.path();
        if path.is_file() && is_executable(&path) {
            files.push(path);
        }
    }
    Ok(files)
}

pub fn when_release_binary_built_for_musl(world: &mut World) -> Result<(), String> {
    let root = workspace_root();
    let status = Command::new("cargo")
        .args([
            "build",
            "--release",
            "--target",
            MUSL_TARGET,
            "-p",
            "trellis-server",
            "--bin",
            "trellis",
        ])
        .current_dir(&root)
        .status()
        .map_err(|e| format!("spawn cargo build: {e}"))?;
    world.release_build_ok = Some(status.success());
    if !status.success() {
        return Err(format!(
            "cargo build --release --target {MUSL_TARGET} exited with {status}"
        ));
    }

    let release_dir = root.join("target").join(MUSL_TARGET).join("release");
    world.release_binaries = Some(executable_files_in(&release_dir)?);
    Ok(())
}

fn release_binaries(world: &World) -> Result<&Vec<PathBuf>, String> {
    world
        .release_binaries
        .as_ref()
        .ok_or_else(|| "release binary was never built".to_string())
}

pub fn then_exactly_one_release_binary(world: &mut World) -> Result<(), String> {
    let binaries = release_binaries(world)?;
    if binaries.len() == 1 {
        Ok(())
    } else {
        Err(format!(
            "expected exactly one release binary, found {}: {binaries:?}",
            binaries.len()
        ))
    }
}

fn description_indicates_static_binary(description: &str) -> bool {
    description.contains("statically linked") || description.contains("static-pie linked")
}

pub fn then_binary_has_no_dynamic_dependencies(world: &mut World) -> Result<(), String> {
    let binaries = release_binaries(world)?;
    let binary = binaries
        .first()
        .ok_or_else(|| "no release binary to inspect".to_string())?;
    let output = Command::new("file")
        .arg(binary)
        .output()
        .map_err(|e| format!("spawn file: {e}"))?;
    let description = String::from_utf8_lossy(&output.stdout);
    if description_indicates_static_binary(&description) {
        Ok(())
    } else {
        Err(format!(
            "expected a statically linked binary, `file` reported: {description}"
        ))
    }
}

pub fn when_scheduler_core_dependency_tree_listed(world: &mut World) -> Result<(), String> {
    let root = workspace_root();
    let output = Command::new("cargo")
        .args(["tree", "-p", "scheduler-core"])
        .current_dir(&root)
        .output()
        .map_err(|e| format!("spawn cargo tree: {e}"))?;
    if !output.status.success() {
        return Err(format!(
            "cargo tree -p scheduler-core exited with {}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    world.cargo_tree_output = Some(String::from_utf8_lossy(&output.stdout).into_owned());
    Ok(())
}

pub fn then_dependency_tree_excludes(world: &mut World, forbidden: &str) -> Result<(), String> {
    let tree = world
        .cargo_tree_output
        .as_ref()
        .ok_or_else(|| "dependency tree was never listed".to_string())?;
    let found = tree.lines().any(|line| {
        let name = line.trim_start_matches(|c: char| !c.is_alphanumeric() && c != '_');
        let name = name.split_whitespace().next().unwrap_or("");
        name == forbidden
    });
    if found {
        Err(format!(
            "dependency tree for scheduler-core contains forbidden dependency \"{forbidden}\""
        ))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn given_workspace_checked_out_passes_for_a_directory_with_a_cargo_toml() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("Cargo.toml"), "[workspace]\n").unwrap();
        assert_eq!(given_workspace_checked_out(dir.path()), Ok(()));
    }

    #[test]
    fn given_workspace_checked_out_errors_without_a_cargo_toml() {
        let dir = tempfile::tempdir().unwrap();
        assert!(given_workspace_checked_out(dir.path()).is_err());
    }

    #[test]
    fn then_exactly_one_release_binary_passes_for_a_single_binary() {
        let mut world = World::new();
        world.release_binaries = Some(vec![PathBuf::from("/tmp/trellis")]);
        assert_eq!(then_exactly_one_release_binary(&mut world), Ok(()));
    }

    #[test]
    fn then_exactly_one_release_binary_errors_when_none_were_produced() {
        let mut world = World::new();
        world.release_binaries = Some(vec![]);
        assert!(then_exactly_one_release_binary(&mut world).is_err());
    }

    #[test]
    fn then_exactly_one_release_binary_errors_when_several_were_produced() {
        let mut world = World::new();
        world.release_binaries = Some(vec![
            PathBuf::from("/tmp/trellis"),
            PathBuf::from("/tmp/trellis-extra"),
        ]);
        assert!(then_exactly_one_release_binary(&mut world).is_err());
    }

    #[test]
    fn then_binary_has_no_dynamic_dependencies_errors_without_a_built_binary() {
        let mut world = World::new();
        world.release_binaries = Some(vec![]);
        assert!(then_binary_has_no_dynamic_dependencies(&mut world).is_err());
    }

    #[test]
    fn description_indicates_static_binary_recognizes_static_reports() {
        assert!(description_indicates_static_binary(
            "ELF 64-bit LSB executable, x86-64, statically linked"
        ));
        assert!(description_indicates_static_binary(
            "ELF 64-bit LSB pie executable, x86-64, static-pie linked"
        ));
    }

    #[test]
    fn description_indicates_static_binary_rejects_dynamic_reports() {
        assert!(!description_indicates_static_binary(
                "ELF 64-bit LSB pie executable, x86-64, dynamically linked, interpreter /lib64/ld-linux-x86-64.so.2"
            ));
    }

    fn tree_with_dependency() -> String {
        "scheduler-core v0.1.0 (/workspace)\n\
             ├── tokio v1.53.1\n\
             └── serde v1.0.229\n"
            .to_string()
    }

    #[test]
    fn then_dependency_tree_excludes_errors_when_the_dependency_is_present() {
        let mut world = World::new();
        world.cargo_tree_output = Some(tree_with_dependency());
        assert!(then_dependency_tree_excludes(&mut world, "tokio").is_err());
    }

    #[test]
    fn then_dependency_tree_excludes_passes_when_the_dependency_is_absent() {
        let mut world = World::new();
        world.cargo_tree_output = Some(tree_with_dependency());
        assert_eq!(then_dependency_tree_excludes(&mut world, "sqlx"), Ok(()));
    }

    #[test]
    fn then_dependency_tree_excludes_does_not_match_a_bare_substring() {
        let mut world = World::new();
        world.cargo_tree_output = Some(tree_with_dependency());
        // "tok" is a substring of "tokio" but not the whole crate name.
        assert_eq!(then_dependency_tree_excludes(&mut world, "tok"), Ok(()));
    }

    #[test]
    fn then_dependency_tree_excludes_errors_without_a_listed_tree() {
        let mut world = World::new();
        assert!(then_dependency_tree_excludes(&mut world, "tokio").is_err());
    }
}
