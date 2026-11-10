//! End-to-end tests driving the `tesh` binary against temporary
//! project directories (std-only; no extra dev-dependencies).

use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static COUNTER: AtomicUsize = AtomicUsize::new(0);

struct TempProject(PathBuf);

impl TempProject {
    fn new() -> TempProject {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let dir = std::env::temp_dir().join(format!(
            "tesh-test-{}-{nanos}-{n}",
            std::process::id()
        ));
        fs::create_dir_all(&dir).unwrap();
        TempProject(dir)
    }

    fn write(&self, rel: &str, content: &str) {
        let path = self.0.join(rel);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, content).unwrap();
    }

    fn read(&self, rel: &str) -> String {
        fs::read_to_string(self.0.join(rel)).unwrap()
    }

    fn exists(&self, rel: &str) -> bool {
        self.0.join(rel).exists()
    }

    fn tesh(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_tesh"))
            .args(args)
            .current_dir(&self.0)
            .output()
            .unwrap()
    }
}

impl Drop for TempProject {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn stdout(out: &Output) -> String {
    String::from_utf8_lossy(&out.stdout).into_owned()
}

fn stderr(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}

const TESH_TOML: &str = r#"
name = "sample-platform"
terraform_binary = "terraform"
output_dir = "output"

[defaults]
strict = true
state_mode = "remote_only"
"#;

const PROD_ENV: &str = r#"
name = "prod"
base = "land"
shrubs = ["waf"]

[context]
provider = "aws"
account = "production"
region = "ap-northeast-1"
backend = "s3://my-tfstate/prod"
"#;

/// A small but complete project: land with two files, one shrub that
/// adds a file and overrides `ec2.tf`.
fn sample_project() -> TempProject {
    let p = TempProject::new();
    p.write("tesh.toml", TESH_TOML);
    p.write("land/vpc.tf", "resource \"aws_vpc\" \"main\" {}\n");
    p.write(
        "land/ec2.tf",
        "resource \"aws_instance\" \"app\" { instance_type = \"c7g.large\" }\n",
    );
    p.write(
        "shrubs/waf/shrub.toml",
        "name = \"waf\"\ndescription = \"WAF v2 rules\"\ntype = \"feature\"\n",
    );
    p.write("shrubs/waf/waf.tf", "resource \"aws_wafv2_web_acl\" \"main\" {}\n");
    p.write(
        "shrubs/waf/ec2.tf",
        "resource \"aws_instance\" \"app\" { instance_type = \"c7g.xlarge\" }\n",
    );
    p.write("envs/prod/env.toml", PROD_ENV);
    p
}

#[test]
fn build_layers_land_then_shrubs_with_provenance() {
    let p = sample_project();
    let out = p.tesh(&["build", "prod"]);
    assert!(out.status.success(), "stderr: {}", stderr(&out));

    // Land-only file: single source.
    let vpc = p.read("output/prod/vpc.tf");
    assert!(vpc.contains("# Source: land/vpc.tf"), "vpc.tf: {vpc}");
    assert!(vpc.contains("aws_vpc"));

    // Shrub-only file.
    let waf = p.read("output/prod/waf.tf");
    assert!(waf.contains("# Source: shrubs/waf/waf.tf"), "waf.tf: {waf}");

    // Overridden file: shrub content wins, both sources are named.
    let ec2 = p.read("output/prod/ec2.tf");
    assert!(
        ec2.contains("# Sources (applied in order; last wins): land/ec2.tf, shrubs/waf/ec2.tf"),
        "ec2.tf: {ec2}"
    );
    assert!(ec2.contains("c7g.xlarge"), "ec2.tf: {ec2}");
    assert!(!ec2.contains("c7g.large\""), "ec2.tf: {ec2}");
}

#[test]
fn build_is_deterministic_and_regenerates() {
    let p = sample_project();
    assert!(p.tesh(&["build", "prod"]).status.success());
    let first = p.read("output/prod/ec2.tf");

    // A hand edit in output/ must vanish on rebuild (design v0.2 §8.4).
    p.write("output/prod/ec2.tf", "hand edited\n");
    p.write("output/prod/stale.tf", "stale\n");
    assert!(p.tesh(&["build", "prod"]).status.success());
    assert_eq!(p.read("output/prod/ec2.tf"), first);
    assert!(!p.exists("output/prod/stale.tf"));
}

#[test]
fn build_rejects_incomplete_context() {
    let p = TempProject::new();
    p.write("tesh.toml", TESH_TOML);
    p.write("land/vpc.tf", "resource \"aws_vpc\" \"main\" {}\n");
    p.write("envs/dev/env.toml", "name = \"dev\"\nbase = \"land\"\nshrubs = []\n");

    let out = p.tesh(&["build", "dev"]);
    assert!(!out.status.success());
    let err = stderr(&out);
    assert!(err.contains("context is incomplete"), "stderr: {err}");
    assert!(err.contains("provider"), "stderr: {err}");
}

#[test]
fn build_rejects_unknown_shrub() {
    let p = TempProject::new();
    p.write("tesh.toml", TESH_TOML);
    p.write("land/vpc.tf", "resource \"aws_vpc\" \"main\" {}\n");
    p.write(
        "envs/dev/env.toml",
        "name = \"dev\"\nshrubs = [\"ghost\"]\n\n[context]\nprovider = \"aws\"\naccount = \"dev\"\nregion = \"ap-northeast-1\"\nbackend = \"s3://state/dev\"\n",
    );

    let out = p.tesh(&["build", "dev"]);
    assert!(!out.status.success());
    assert!(stderr(&out).contains("ghost"), "stderr: {}", stderr(&out));
}

#[test]
fn newenv_newshrub_add_shrub_flow() {
    let p = TempProject::new();
    p.write("tesh.toml", TESH_TOML);
    p.write("land/vpc.tf", "resource \"aws_vpc\" \"main\" {}\n");

    let out = p.tesh(&["newshrub", "waf", "--type", "feature"]);
    assert!(out.status.success(), "stderr: {}", stderr(&out));
    assert!(p.exists("shrubs/waf/shrub.toml"));

    let out = p.tesh(&["newenv", "dev"]);
    assert!(out.status.success(), "stderr: {}", stderr(&out));
    assert!(p.exists("envs/dev/env.toml"));

    let out = p.tesh(&["env", "add-shrub", "dev", "waf"]);
    assert!(out.status.success(), "stderr: {}", stderr(&out));
    assert!(
        p.read("envs/dev/env.toml").contains("\"waf\""),
        "env.toml: {}",
        p.read("envs/dev/env.toml")
    );

    // Adding the same shrub twice is an error.
    let out = p.tesh(&["env", "add-shrub", "dev", "waf"]);
    assert!(!out.status.success());
    assert!(stderr(&out).contains("already has shrub"));

    // Creating an existing env is an error.
    let out = p.tesh(&["newenv", "dev"]);
    assert!(!out.status.success());
}

#[test]
fn newenv_from_copies_shrubs_and_context() {
    let p = sample_project();
    let out = p.tesh(&["newenv", "prod2", "--from", "prod"]);
    assert!(out.status.success(), "stderr: {}", stderr(&out));
    let text = p.read("envs/prod2/env.toml");
    assert!(text.contains("\"waf\""), "env.toml: {text}");
    assert!(text.contains("ap-northeast-1"), "env.toml: {text}");
    assert!(text.contains("name = \"prod2\""), "env.toml: {text}");
}

#[test]
fn doctor_reports_problems_and_passes_clean_projects() {
    // Clean project (prod's context is complete, waf is used).
    let p = sample_project();
    let out = p.tesh(&["doctor"]);
    assert!(out.status.success(), "stdout: {}", stdout(&out));
    assert!(stdout(&out).contains("no problems found"));

    // Broken project: unknown shrub reference + incomplete context.
    let p2 = TempProject::new();
    p2.write("tesh.toml", TESH_TOML);
    p2.write(
        "envs/dev/env.toml",
        "name = \"dev\"\nshrubs = [\"ghost\"]\n",
    );
    let out = p2.tesh(&["doctor"]);
    assert!(!out.status.success());
    let text = stdout(&out);
    assert!(text.contains("ghost"), "stdout: {text}");
    assert!(text.contains("context is incomplete"), "stdout: {text}");
}

#[test]
fn unimplemented_commands_say_so() {
    let p = sample_project();
    for args in [
        vec!["plan", "prod"],
        vec!["apply", "prod"],
        vec!["diff", "prod", "prod"],
        vec!["explain", "prod", "module.app"],
        vec!["promote", "shrub", "waf"],
    ] {
        let out = p.tesh(&args);
        assert!(!out.status.success(), "args: {args:?}");
        assert!(
            stderr(&out).contains("not yet implemented"),
            "args: {args:?}, stderr: {}",
            stderr(&out)
        );
    }
}

#[test]
fn running_outside_a_project_is_an_error() {
    let p = TempProject::new();
    let out = p.tesh(&["doctor"]);
    assert!(!out.status.success());
    assert!(stderr(&out).contains("no tesh.toml found"));
}
