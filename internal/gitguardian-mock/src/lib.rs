#![cfg(unix)]

mod patch;

pub mod prefer;

use patch::patch;

use std::fs::File;
use std::io::{BufRead, BufReader};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::OnceLock;
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::time::Duration;

use serde_json::Value;

const SPEC_URL: &str = "https://api.gitguardian.com/v1/openapi.json";
const PRISM: &str = "npx";
const PRISM_ARGS: [&str; 2] = ["-y", "@stoplight/prism-cli@5"];
const READY_MARKER: &str = "Prism is listening on";
const START_TIMEOUT: Duration = Duration::from_secs(300);

pub struct MockServer {
    child: Child,
    uri: String,
    spec_path: PathBuf,
}

impl MockServer {
    pub fn shared() -> &'static Self {
        static SHARED: OnceLock<MockServer> = OnceLock::new();
        SHARED.get_or_init(|| {
            let server = Self::start().expect("failed to start the prism mock server");
            register_shared_cleanup(server.child.id(), &server.spec_path);
            server
        })
    }

    fn start() -> std::io::Result<Self> {
        let port = free_port()?;

        let mut spec = load_spec(SPEC_URL)?;
        patch(&mut spec);
        let spec_path = std::env::temp_dir().join(format!("gitguardian-mock-{port}.json"));
        serde_json::to_writer(File::create(&spec_path)?, &spec).map_err(std::io::Error::other)?;

        let mut builder = Command::new(PRISM);
        builder
            .args(PRISM_ARGS)
            .arg("mock")
            .arg("-h")
            .arg("127.0.0.1")
            .arg("-p")
            .arg(port.to_string())
            .arg(&spec_path)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        detach_process_group(&mut builder);

        let mut child = match builder.spawn() {
            Ok(child) => child,
            Err(error) => {
                let _ = std::fs::remove_file(&spec_path);
                return Err(error);
            }
        };

        let stdout = child.stdout.take().expect("stdout was piped");
        let (sender, receiver) = mpsc::channel();
        std::thread::spawn(move || {
            let mut sender = Some(sender);
            for line in BufReader::new(stdout).lines().map_while(Result::ok) {
                if line.contains(READY_MARKER)
                    && let Some(sender) = sender.take()
                {
                    let _ = sender.send(());
                }
            }
        });

        match receiver.recv_timeout(START_TIMEOUT) {
            Ok(()) => Ok(Self { child, uri: format!("http://127.0.0.1:{port}"), spec_path }),
            Err(error) => {
                terminate(child.id());
                let _ = child.kill();
                let _ = child.wait();
                let _ = std::fs::remove_file(&spec_path);
                Err(std::io::Error::other(match error {
                    RecvTimeoutError::Timeout => "timed out waiting for the prism mock server",
                    RecvTimeoutError::Disconnected => {
                        "the prism mock server exited before it was ready"
                    }
                }))
            }
        }
    }

    pub fn uri(&self) -> &str {
        &self.uri
    }

    pub fn base_uri(&self) -> String {
        format!("{}/v1/", self.uri)
    }
}

impl Drop for MockServer {
    fn drop(&mut self) {
        terminate(self.child.id());
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = std::fs::remove_file(&self.spec_path);
    }
}

fn load_spec(source: &str) -> std::io::Result<Value> {
    let text = if source.starts_with("http://") || source.starts_with("https://") {
        let mut response = ureq::get(source).call().map_err(std::io::Error::other)?;
        response
            .body_mut()
            .with_config()
            .limit(64 * 1024 * 1024)
            .read_to_string()
            .map_err(std::io::Error::other)?
    } else {
        std::fs::read_to_string(source)?
    };
    serde_json::from_str(&text).map_err(std::io::Error::other)
}

fn free_port() -> std::io::Result<u16> {
    Ok(TcpListener::bind("127.0.0.1:0")?.local_addr()?.port())
}

fn detach_process_group(builder: &mut Command) {
    std::os::unix::process::CommandExt::process_group(builder, 0);
}

fn terminate(pid: u32) {
    unsafe { libc::kill(-(pid as i32), libc::SIGKILL) };
}

static SHARED_PID: AtomicI32 = AtomicI32::new(0);

static SHARED_SPEC: OnceLock<PathBuf> = OnceLock::new();

fn register_shared_cleanup(pid: u32, spec_path: &Path) {
    SHARED_PID.store(pid as i32, Ordering::SeqCst);
    let _ = SHARED_SPEC.set(spec_path.to_path_buf());
    unsafe { libc::atexit(cleanup_shared) };
}

extern "C" fn cleanup_shared() {
    let pid = SHARED_PID.load(Ordering::SeqCst);
    if pid != 0 {
        terminate(pid as u32);
    }
    if let Some(spec_path) = SHARED_SPEC.get() {
        let _ = std::fs::remove_file(spec_path);
    }
}
