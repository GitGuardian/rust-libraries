mod support;

use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::Barrier;
use std::time::{Duration, Instant};
use std::{env, fs, process, thread};

use gg_fs::{Cause, FileLock, read, write};
use support::Scratch;

const HOLDER_ENV: &str = "GG_FS_TEST_LOCK_HOLDER";
const LONG: Duration = Duration::from_secs(10);

/// Holds the lock named by the environment until the process is killed. It does
/// nothing when run as a regular test.
#[test]
fn lock_holder_process() {
    if let Ok(path) = env::var(HOLDER_ENV) {
        let _lock = FileLock::lock(path, Duration::ZERO).unwrap();
        println!("locked");
        std::io::stdout().flush().unwrap();
        thread::sleep(Duration::from_secs(120));
    }
}

struct Holder(Child);

impl Holder {
    fn kill(&mut self) {
        self.0.kill().unwrap();
        self.0.wait().unwrap();
    }
}

impl Drop for Holder {
    fn drop(&mut self) {
        self.0.kill().ok();
    }
}

fn spawn_holder(path: &Path) -> Holder {
    let mut child = Command::new(env::current_exe().unwrap())
        .args(["--exact", "lock_holder_process", "--nocapture", "--test-threads=1"])
        .env(HOLDER_ENV, path)
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let stdout = child.stdout.take().unwrap();
    let holder = Holder(child);
    let locked =
        BufReader::new(stdout).lines().map_while(Result::ok).any(|line| line.ends_with("locked"));
    assert!(locked, "the holder process did not take the lock");
    holder
}

/// GIVEN one holder
/// WHEN a second tries to lock without waiting
/// THEN it is refused
#[test]
fn lock_is_exclusive_between_holders() {
    let scratch = Scratch::new();
    let path = scratch.path("a.lock");
    let _first = FileLock::lock(&path, Duration::ZERO).unwrap();

    let err = FileLock::lock(&path, Duration::ZERO).err().unwrap();

    assert!(matches!(err.cause(), Cause::Timeout(timeout) if timeout.is_zero()));
    assert_eq!(err.path(), path);
}

/// GIVEN a held lock
/// WHEN the guard is dropped
/// THEN another holder can lock
#[test]
fn lock_is_released_on_drop() {
    let scratch = Scratch::new();
    let path = scratch.path("a.lock");
    let first = FileLock::lock(&path, Duration::ZERO).unwrap();

    drop(first);

    let _second = FileLock::lock(&path, Duration::ZERO).unwrap();
}

/// GIVEN a lock held by another process
/// WHEN this process locks
/// THEN it is refused until that process is killed, and then it succeeds
#[test]
fn lock_is_exclusive_across_processes_and_released_when_one_dies() {
    let scratch = Scratch::new();
    let path = scratch.path("a.lock");
    let mut holder = spawn_holder(&path);

    let refused = FileLock::lock(&path, Duration::ZERO).err();
    holder.kill();
    let acquired = FileLock::lock(&path, LONG);

    assert!(matches!(refused.unwrap().cause(), Cause::Timeout(_)));
    let _lock = acquired.unwrap();
}

/// GIVEN a lock held elsewhere
/// WHEN a lock with a timeout is requested
/// THEN it polls and then fails with a timeout error
#[test]
fn lock_waits_until_timeout() {
    let scratch = Scratch::new();
    let path = scratch.path("a.lock");
    let _held = FileLock::lock(&path, Duration::ZERO).unwrap();
    let timeout = Duration::from_millis(300);
    let started = Instant::now();

    let err = FileLock::lock(&path, timeout).err().unwrap();

    assert!(started.elapsed() >= timeout);
    assert!(matches!(err.cause(), Cause::Timeout(reported) if *reported == timeout));
}

/// GIVEN a lock released during the wait
/// WHEN a lock with a timeout is requested
/// THEN it succeeds
#[test]
fn lock_acquires_when_released_during_wait() {
    let scratch = Scratch::new();
    let path = scratch.path("a.lock");
    let held = FileLock::lock(&path, Duration::ZERO).unwrap();

    thread::scope(|scope| {
        scope.spawn(move || {
            thread::sleep(Duration::from_millis(200));
            drop(held);
        });
        let _lock = FileLock::lock(&path, LONG).unwrap();
    });
}

/// GIVEN a lock file unlinked by its previous holder
/// WHEN a waiting holder acquires it
/// THEN the unlinked file is detected and the path is locked again
#[cfg(unix)]
#[test]
fn lock_rejects_file_unlinked_by_previous_holder() {
    let scratch = Scratch::new();
    let path = scratch.path("a.lock");
    let held = FileLock::lock(&path, Duration::ZERO).unwrap();

    thread::scope(|scope| {
        let waiter = scope.spawn(|| FileLock::lock(&path, LONG).map(|lock| (lock, path.exists())));
        thread::sleep(Duration::from_millis(150));
        fs::remove_file(&path).unwrap();
        drop(held);
        let (_lock, exists) = waiter.join().unwrap().unwrap();
        assert!(exists);
    });
}

/// GIVEN a held lock
/// WHEN the lock file is deleted
/// THEN the delete fails
#[cfg(windows)]
#[test]
fn held_lock_file_cannot_be_deleted() {
    let scratch = Scratch::new();
    let path = scratch.path("a.lock");
    let _held = FileLock::lock(&path, Duration::ZERO).unwrap();

    assert!(fs::remove_file(&path).is_err());
    assert!(path.exists());
}

/// GIVEN a symlink at the lock path
/// WHEN the lock is requested
/// THEN an error is returned
#[cfg(unix)]
#[test]
fn lock_file_is_opened_without_following_symlinks() {
    let scratch = Scratch::new();
    let target = scratch.write("target", "");
    let path = scratch.path("a.lock");
    support::unix::link(&target, &path);

    assert!(FileLock::lock(&path, Duration::ZERO).is_err());
}

/// GIVEN no lock directory
/// WHEN the lock is requested
/// THEN the parent is created 0700 and the lock file 0600
#[cfg(unix)]
#[test]
fn lock_creates_missing_parent_directory_privately() {
    let scratch = Scratch::new();
    let path = scratch.path("locks/nested/a.lock");

    let _lock = FileLock::lock(&path, Duration::ZERO).unwrap();

    assert_eq!(support::unix::mode_of(&scratch.path("locks")), 0o700);
    assert_eq!(support::unix::mode_of(&scratch.path("locks/nested")), 0o700);
    assert_eq!(support::unix::mode_of(&path), 0o600);
}

/// GIVEN a released lock
/// WHEN the lock file is read
/// THEN it contains the process id of the holder
#[test]
fn lock_file_records_the_holder_pid() {
    let scratch = Scratch::new();
    let path = scratch.path("a.lock");

    drop(FileLock::lock(&path, Duration::ZERO).unwrap());

    assert_eq!(fs::read_to_string(&path).unwrap(), process::id().to_string());
}

/// GIVEN several launchers racing on one lock file
/// WHEN all start together
/// THEN exactly one acquires it
#[test]
fn singleton_file_prevents_two_launchers() {
    const LAUNCHERS: usize = 6;
    let scratch = Scratch::new();
    let path = scratch.path("launcher.pid");
    let start = Barrier::new(LAUNCHERS);
    let finished = Barrier::new(LAUNCHERS);

    let winners: usize = thread::scope(|scope| {
        let handles: Vec<_> = (0..LAUNCHERS)
            .map(|_| {
                scope.spawn(|| {
                    start.wait();
                    let lock = FileLock::lock(&path, Duration::ZERO);
                    finished.wait();
                    usize::from(lock.is_ok())
                })
            })
            .collect();
        handles.into_iter().map(|handle| handle.join().unwrap()).sum()
    });

    assert_eq!(winners, 1);
}

/// GIVEN several threads updating one file
/// WHEN each runs a locked read-modify-write cycle
/// THEN every update is preserved
#[test]
fn read_modify_write_cycle_holds_the_lock() {
    const WORKERS: usize = 6;
    const ROUNDS: usize = 10;
    let scratch = Scratch::new();
    let counter = scratch.write("counter", "0");
    let lock_path = scratch.path("counter.lock");

    thread::scope(|scope| {
        for _ in 0..WORKERS {
            scope.spawn(|| {
                for _ in 0..ROUNDS {
                    let _lock = FileLock::lock(&lock_path, LONG).unwrap();
                    let value: usize =
                        String::from_utf8(read(&counter).unwrap()).unwrap().parse().unwrap();
                    write(&counter, (value + 1).to_string()).unwrap();
                }
            });
        }
    });

    assert_eq!(fs::read_to_string(&counter).unwrap(), (WORKERS * ROUNDS).to_string());
}
