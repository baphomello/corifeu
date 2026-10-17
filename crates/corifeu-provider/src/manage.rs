//! Owning a local runtime: start it once, never twice, and stop it again.
//!
//! Until this existed nothing owned the server. It was started by hand and
//! nothing prevented a second one, which is not a theoretical cost: a 9B with
//! its weights entirely on the GPU still holds about 8 GB of host RAM, so two
//! servers for one endpoint is 16 GB on a 32 GB machine and the first symptom
//! is the desktop swapping while the GPU sits half empty. It happened twice,
//! and both times the fix had been written down rather than built.
//!
//! # What makes one instance one instance
//!
//! Two things, because neither is enough alone.
//!
//! **The probe.** Before starting anything, ask the endpoint whether it
//! answers. One that does belongs to somebody — the user, an earlier run, a
//! machine that is not this one — and Corifeu neither restarts nor stops it.
//!
//! **The lock.** The probe loses a race: two `corifeu` processes can both find
//! nothing and both start a server. The lock is a file named after the
//! endpoint, created exclusively, carrying the pid of the Corifeu that holds
//! it. A lock whose owner is gone is stale and gets taken.
//!
//! # Why stopping takes a command
//!
//! The obvious implementation kills the child. That works where the runtime is
//! a direct child of Corifeu, and not where it is reached across a boundary —
//! Corifeu in WSL starting a server on the Windows host gets a launcher that
//! exits immediately, and the server it spawned is nobody's child. So a plan
//! may carry its own `stop`, and whether Corifeu started the runtime is
//! recorded in the lock rather than inferred from a live process handle.

use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("`{0}` is not a command")]
    NoCommand(String),

    #[error("could not start {program}: {source}")]
    Spawn {
        program: String,
        #[source]
        source: std::io::Error,
    },

    #[error(
        "{base_url} did not answer within {seconds}s of starting `{program}`. \
         The runtime's own output is the next place to look."
    )]
    NeverReady {
        base_url: String,
        program: String,
        seconds: u64,
    },

    #[error(
        "another corifeu (pid {pid}) is starting a runtime for {base_url}. \
         Wait for it, or stop it."
    )]
    Held { pid: u32, base_url: String },

    #[error("could not take the lock at {path}: {source}")]
    Lock {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}

/// How to bring one endpoint up.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Plan {
    pub base_url: String,
    /// The argument vector that starts the runtime. No shell (ADR-0037).
    pub command: Vec<String>,
    /// What stops it, where killing a child would not. Absent means the child
    /// Corifeu spawned is the runtime, and killing it is enough.
    pub stop: Option<Vec<String>>,
    /// Seconds of idleness after which the runtime releases its memory. Zero
    /// leaves the runtime's own default alone.
    pub sleep_idle_s: u32,
    pub stop_when_done: bool,
    pub ready_timeout: Duration,
}

/// Who the runtime belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Found {
    /// It was already answering. Corifeu did not start it and will not stop
    /// it: an endpoint that answers is somebody's, and guessing whose is how a
    /// tool kills the server a person was using.
    Already,
    /// Corifeu started it.
    Started,
}

/// A runtime Corifeu is responsible for, for as long as this value lives.
pub struct Runtime {
    plan: Plan,
    found: Found,
    child: Option<Child>,
    lock: Option<PathBuf>,
}

impl Runtime {
    /// Make the endpoint answer, starting it only if nothing does.
    pub fn ensure(plan: Plan, probe: &dyn Fn(&str) -> bool) -> Result<Self, Error> {
        if probe(&plan.base_url) {
            return Ok(Self {
                plan,
                found: Found::Already,
                child: None,
                lock: None,
            });
        }

        let path = lock_path(&plan.base_url);
        let lock = take(&path, &plan.base_url)?;

        // Between the probe and the lock, whoever held the lock may have
        // finished starting one. Ask again rather than start a second.
        if probe(&plan.base_url) {
            let _ = fs::remove_file(&lock);
            return Ok(Self {
                plan,
                found: Found::Already,
                child: None,
                lock: None,
            });
        }

        let argv = argv_with_sleep(&plan);
        let Some((program, rest)) = argv.split_first() else {
            let _ = fs::remove_file(&lock);
            return Err(Error::NoCommand(plan.base_url.clone()));
        };

        let child = Command::new(program)
            .args(rest)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|source| Error::Spawn {
                program: program.clone(),
                source,
            });

        let child = match child {
            Ok(child) => child,
            Err(error) => {
                let _ = fs::remove_file(&lock);
                return Err(error);
            }
        };

        let mut runtime = Self {
            found: Found::Started,
            child: Some(child),
            lock: Some(lock),
            plan,
        };

        let deadline = Instant::now() + runtime.plan.ready_timeout;
        while Instant::now() < deadline {
            if probe(&runtime.plan.base_url) {
                return Ok(runtime);
            }
            std::thread::sleep(Duration::from_millis(500));
        }

        let seconds = runtime.plan.ready_timeout.as_secs();
        let program = program.clone();
        let base_url = runtime.plan.base_url.clone();
        runtime.stop();
        Err(Error::NeverReady {
            base_url,
            program,
            seconds,
        })
    }

    pub fn found(&self) -> Found {
        self.found
    }

    /// Stop it, if Corifeu started it and was asked to.
    pub fn stop(&mut self) {
        if self.found == Found::Started && self.plan.stop_when_done {
            match self.plan.stop.as_deref() {
                Some(argv) => {
                    if let Some((program, rest)) = argv.split_first() {
                        let _ = Command::new(program)
                            .args(rest)
                            .stdin(Stdio::null())
                            .stdout(Stdio::null())
                            .stderr(Stdio::null())
                            .status();
                    }
                }
                None => {
                    if let Some(child) = self.child.as_mut() {
                        let _ = child.kill();
                        let _ = child.wait();
                    }
                }
            }
        }
        self.child = None;
        if let Some(path) = self.lock.take() {
            let _ = fs::remove_file(path);
        }
    }
}

impl Drop for Runtime {
    fn drop(&mut self) {
        self.stop();
    }
}

pub const SLEEP_FLAG: &str = "--sleep-idle-seconds";

/// Whether this command already decides its own idle sleep.
///
/// A substring and not an argv element, because the flag is not always an
/// argument of the program being run: a command that reaches the runtime
/// across a boundary carries the runtime's whole argument list inside one
/// string, and appending to the launcher's argv would pass the flag to the
/// launcher.
pub fn names_sleep(command: &[String]) -> bool {
    command.iter().any(|a| a.contains(SLEEP_FLAG))
}

/// The command, with the idle-sleep flag added when it is wanted and absent.
///
/// Appended rather than required, because the flag is the whole point: a
/// runtime that releases its memory after five idle minutes makes forgetting
/// to stop it cost nothing, and a user who has to remember a flag is back
/// where this module started. A command that already names it keeps what it
/// says, and `sleep_idle_s: 0` leaves it alone.
pub fn argv_with_sleep(plan: &Plan) -> Vec<String> {
    let mut argv = plan.command.clone();
    if plan.sleep_idle_s > 0 && !names_sleep(&argv) {
        argv.push(SLEEP_FLAG.to_string());
        argv.push(plan.sleep_idle_s.to_string());
    }
    argv
}

/// One lock per endpoint, named after it. Two projects pointing at the same
/// server share the lock, which is correct: they share the server.
pub fn lock_path(base_url: &str) -> PathBuf {
    use std::hash::{DefaultHasher, Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    base_url.trim_end_matches('/').hash(&mut hasher);
    let dir = std::env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join("corifeu");
    dir.join(format!("runtime-{:016x}.lock", hasher.finish()))
}

fn take(path: &Path, base_url: &str) -> Result<PathBuf, Error> {
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let mine = std::process::id();

    match fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
    {
        Ok(_) => {
            let _ = fs::write(path, mine.to_string());
            Ok(path.to_path_buf())
        }
        Err(e) if e.kind() == ErrorKind::AlreadyExists => {
            let held = fs::read_to_string(path)
                .ok()
                .and_then(|s| s.trim().parse::<u32>().ok());
            match held {
                // A lock whose owner is gone is a crash, not an owner.
                Some(pid) if alive(pid) => Err(Error::Held {
                    pid,
                    base_url: base_url.to_string(),
                }),
                _ => {
                    let _ = fs::write(path, mine.to_string());
                    Ok(path.to_path_buf())
                }
            }
        }
        Err(source) => Err(Error::Lock {
            path: path.to_path_buf(),
            source,
        }),
    }
}

/// Whether a pid is a process that still exists.
///
/// `/proc` rather than `kill(pid, 0)` so this needs no libc, and it answers
/// the only question the lock asks. On a platform without `/proc` the answer
/// is "assume it is alive", which keeps the lock conservative: refusing to
/// start is recoverable and starting a second server is what this module
/// exists to prevent.
fn alive(pid: u32) -> bool {
    if cfg!(target_os = "linux") {
        return Path::new(&format!("/proc/{pid}")).exists();
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plan(base_url: &str) -> Plan {
        Plan {
            base_url: base_url.to_string(),
            command: vec!["true".into()],
            stop: None,
            sleep_idle_s: 0,
            stop_when_done: true,
            ready_timeout: Duration::from_millis(300),
        }
    }

    /// The case that matters most: a server somebody else is using is not
    /// restarted, and above all is not stopped when this run ends.
    #[test]
    fn a_runtime_that_already_answers_is_left_alone() {
        let runtime =
            Runtime::ensure(plan("http://ja-responde"), &|_| true).expect("probe says yes");
        assert_eq!(runtime.found(), Found::Already);
        assert!(runtime.child.is_none());
        assert!(
            runtime.lock.is_none(),
            "nothing was locked, nothing is ours"
        );
    }

    #[test]
    fn a_runtime_that_never_answers_is_an_error_naming_the_wait() {
        let failed = match Runtime::ensure(plan("http://nunca-responde"), &|_| false) {
            Err(failed) => failed,
            Ok(_) => panic!("um endereço que nunca responde não pode dar certo"),
        };
        let said = failed.to_string();
        assert!(said.contains("nunca-responde"), "{said}");
        assert!(said.contains("0s") || said.contains("within"), "{said}");
    }

    /// The lock is removed on the way out, or the next run finds a stale one
    /// and has to reason about it.
    #[test]
    fn a_failed_start_leaves_no_lock_behind() {
        let base = "http://sem-lock-pendurado";
        let _ = Runtime::ensure(plan(base), &|_| false);
        assert!(!lock_path(base).exists());
    }

    #[test]
    fn two_endpoints_take_two_locks_and_one_endpoint_one() {
        assert_ne!(lock_path("http://a:1"), lock_path("http://b:2"));
        assert_eq!(lock_path("http://a:1/"), lock_path("http://a:1"));
    }

    /// A pid nothing owns is a crashed run, not an owner, or a machine that
    /// rebooted mid-round would need the lock deleted by hand.
    #[test]
    fn a_lock_whose_owner_is_gone_is_taken() {
        let path = std::env::temp_dir().join("corifeu-teste-lock-morto.lock");
        let _ = fs::remove_file(&path);
        fs::write(&path, "4294967295").expect("escreve o lock");
        take(&path, "http://x").expect("um dono morto não segura o lock");
        assert_eq!(
            fs::read_to_string(&path).unwrap().trim(),
            std::process::id().to_string()
        );
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn the_sleep_flag_is_added_once_and_only_when_wanted() {
        let mut p = plan("http://x");
        p.sleep_idle_s = 300;
        assert_eq!(
            argv_with_sleep(&p),
            vec!["true", "--sleep-idle-seconds", "300"]
        );

        p.sleep_idle_s = 0;
        assert_eq!(argv_with_sleep(&p), vec!["true"]);

        // A command that already says it keeps what it says.
        p.sleep_idle_s = 300;
        p.command = vec!["true".into(), "--sleep-idle-seconds".into(), "60".into()];
        assert_eq!(
            argv_with_sleep(&p),
            vec!["true", "--sleep-idle-seconds", "60"]
        );

        // E quando o runtime é alcançado por um lançador, o flag está dentro
        // de uma string só. Acrescentar ao argv do lançador passaria o flag
        // para o lançador.
        p.command = vec![
            "powershell.exe".into(),
            "-Command".into(),
            "Start-Process llama -ArgumentList @('serve','--sleep-idle-seconds','120')".into(),
        ];
        assert_eq!(argv_with_sleep(&p), p.command, "o comando decide sozinho");
    }
}
