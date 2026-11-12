//! A throw-away PostgreSQL cluster for the tests (PG-001 §10): `initdb` and `pg_ctl` in a
//! temporary directory, reachable only over a Unix socket in that directory. When the machine
//! has no PostgreSQL binaries the tests say so and skip. Run as root (containers), the server runs
//! as the `postgres` user through `runuser`, because PostgreSQL refuses to run as root.

use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

use crate::StoreConfig;

static NEXT: AtomicU32 = AtomicU32::new(0);

pub struct TestDb {
    dir: PathBuf,
    bin: PathBuf,
    as_postgres: bool,
}

fn bin_dir() -> Option<PathBuf> {
    if let Some(dir) = std::env::var_os("CFA4_PG_BIN") {
        return Some(PathBuf::from(dir));
    }
    let mut versions: Vec<PathBuf> =
        std::fs::read_dir("/usr/lib/postgresql").ok()?.flatten().map(|e| e.path().join("bin")).filter(|p| p.join("initdb").exists()).collect();
    versions.sort();
    versions.pop()
}

fn is_root() -> bool {
    Command::new("id").arg("-u").output().ok().is_some_and(|o| o.stdout.trim_ascii() == b"0")
}

impl TestDb {
    fn command(&self, tool: &str) -> Command {
        let exe = self.bin.join(tool);
        if self.as_postgres {
            let mut c = Command::new("runuser");
            c.args(["-u", "postgres", "--"]).arg(exe);
            c
        } else {
            Command::new(exe)
        }
    }

    /// Start a cluster, or `None` (with a note) when PostgreSQL is not installed.
    pub fn start() -> Option<TestDb> {
        let Some(bin) = bin_dir() else {
            eprintln!("cfa4-store tests: no PostgreSQL binaries (set CFA4_PG_BIN); skipped");
            return None;
        };
        let n = NEXT.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("cfa4-store-{}-{n}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).ok()?;
        let as_postgres = is_root();
        if as_postgres {
            let ok = Command::new("chown").args(["-R", "postgres:postgres"]).arg(&dir).status().ok().is_some_and(|s| s.success());
            if !ok {
                eprintln!("cfa4-store tests: running as root and cannot hand the directory to `postgres`; skipped");
                return None;
            }
        }
        let db = TestDb { dir, bin, as_postgres };
        let data = db.dir.join("data");
        let initdb = db.command("initdb").arg("-D").arg(&data).args(["-U", "cfa4", "-A", "trust", "--no-sync", "-E", "UTF8"]).output().ok()?;
        if !initdb.status.success() {
            eprintln!("cfa4-store tests: initdb failed: {}; skipped", String::from_utf8_lossy(&initdb.stderr));
            return None;
        }
        let opts = format!("-k {} -c listen_addresses='' -c fsync=off -c full_page_writes=off", db.dir.display());
        let start = db.command("pg_ctl").arg("-D").arg(&data).args(["-w", "-o", &opts, "-l"]).arg(db.dir.join("log")).arg("start").output().ok()?;
        if !start.status.success() {
            eprintln!("cfa4-store tests: pg_ctl start failed: {}; skipped", String::from_utf8_lossy(&start.stderr));
            return None;
        }
        Some(db)
    }

    pub fn config(&self) -> StoreConfig {
        StoreConfig { host: self.dir.display().to_string(), port: 5432, user: "cfa4".into(), dbname: "postgres".into(), ..StoreConfig::default() }
    }
}

impl Drop for TestDb {
    fn drop(&mut self) {
        let _ = self.command("pg_ctl").arg("-D").arg(self.dir.join("data")).args(["-m", "immediate", "stop"]).output();
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}
