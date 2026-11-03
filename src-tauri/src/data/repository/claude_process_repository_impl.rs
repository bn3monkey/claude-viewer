use std::collections::HashMap;
use std::process::Stdio;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::runtime::Handle;
use tokio::sync::mpsc::{unbounded_channel, UnboundedSender};

use crate::data::datasource::stream_json::{decode_line, encode_permission_response, encode_user_message};
use crate::domain::entity::{AppError, ChatEvent};
use crate::domain::repository::{ClaudeProcessRepository, ProcessStatus, SessionStart, SpawnSpec};
use crate::infrastructure::process::{kill_tree, tokio_command};

struct Running {
    /// 같은 세션을 kill 후 다시 띄웠을 때 이전 프로세스의 종료 알림을 구분한다.
    generation: u64,
    pid: Option<u32>,
    stdin: UnboundedSender<String>,
    busy: bool,
    model: String,
    permission_mode: Option<String>,
}

#[derive(Default)]
struct Shared {
    running: Mutex<HashMap<String, Running>>,
    aborted: Mutex<Vec<u64>>,
    stopped_at: Mutex<HashMap<String, f64>>,
    subscribers: Mutex<Vec<Sender<(String, ChatEvent)>>>,
}

impl Shared {
    fn mark_stopped(&self, session_id: &str) {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).map_or(0.0, |d| d.as_millis() as f64);
        self.stopped_at.lock().unwrap().insert(session_id.to_string(), now);
    }

    fn publish(&self, session_id: &str, event: ChatEvent) {
        self.subscribers
            .lock()
            .unwrap()
            .retain(|tx| tx.send((session_id.to_string(), event.clone())).is_ok());
    }

    fn set_busy(&self, session_id: &str, generation: u64, busy: bool) {
        if let Some(entry) = self.running.lock().unwrap().get_mut(session_id) {
            if entry.generation == generation {
                entry.busy = busy;
            }
        }
    }
}

pub struct ClaudeProcessRepositoryImpl {
    runtime: Handle,
    shared: Arc<Shared>,
    next_generation: AtomicU64,
}

impl ClaudeProcessRepositoryImpl {
    pub fn new(runtime: Handle) -> Self {
        Self { runtime, shared: Arc::default(), next_generation: AtomicU64::new(1) }
    }
}

impl ClaudeProcessRepository for ClaudeProcessRepositoryImpl {
    fn spawn(&self, spec: SpawnSpec) -> Result<(), AppError> {
        let mut command = tokio_command(&spec.binary);
        command
            .current_dir(&spec.cwd)
            .args(["-p", "--output-format", "stream-json", "--input-format", "stream-json", "--verbose"])
            // 권한 질문을 stdout 의 control_request 로 받는다. 없으면 질문이 전부 거부된다.
            .args(["--permission-prompt-tool", "stdio"])
            .args(["--model", &spec.model])
            .args(["--append-system-prompt", &spec.append_system_prompt]);
        match spec.start {
            SessionStart::New => command.args(["--session-id", &spec.session_id]),
            SessionStart::Resume => command.args(["--resume", &spec.session_id]),
        };
        if let Some(mode) = &spec.permission_mode {
            command.args(["--permission-mode", mode]);
        }
        command.stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped());

        // tokio::process 는 런타임 안에서 만들어야 한다.
        let _guard = self.runtime.enter();
        let mut child = command
            .spawn()
            .map_err(|e| AppError::Process(format!("claude 실행 실패 ({}): {e}", spec.binary)))?;

        let generation = self.next_generation.fetch_add(1, Ordering::Relaxed);
        let (stdin_tx, mut stdin_rx) = unbounded_channel::<String>();
        let mut stdin = child.stdin.take().expect("stdin piped");
        let stdout = child.stdout.take().expect("stdout piped");
        let stderr = child.stderr.take().expect("stderr piped");

        self.shared.running.lock().unwrap().insert(
            spec.session_id.clone(),
            Running {
                generation,
                pid: child.id(),
                stdin: stdin_tx,
                busy: false,
                model: spec.model.clone(),
                permission_mode: spec.permission_mode.clone(),
            },
        );

        self.runtime.spawn(async move {
            while let Some(line) = stdin_rx.recv().await {
                if stdin.write_all(line.as_bytes()).await.is_err() || stdin.flush().await.is_err() {
                    break;
                }
            }
        });

        let shared = self.shared.clone();
        let id = spec.session_id.clone();
        self.runtime.spawn(async move {
            let mut lines = BufReader::new(stderr).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                if !line.trim().is_empty() {
                    shared.publish(&id, ChatEvent::Error { message: line });
                }
            }
        });

        let shared = self.shared.clone();
        let id = spec.session_id;
        self.runtime.spawn(async move {
            let mut lines = BufReader::new(stdout).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                for event in decode_line(&line) {
                    if matches!(event, ChatEvent::Result { .. }) {
                        shared.set_busy(&id, generation, false);
                    }
                    shared.publish(&id, event);
                }
            }
            let code = child.wait().await.ok().and_then(|status| status.code());
            let aborted = {
                let mut aborted = shared.aborted.lock().unwrap();
                let was = aborted.contains(&generation);
                aborted.retain(|g| *g != generation);
                was
            };
            {
                let mut running = shared.running.lock().unwrap();
                if running.get(&id).map_or(false, |entry| entry.generation == generation) {
                    running.remove(&id);
                    shared.mark_stopped(&id);
                }
            }
            shared.publish(&id, ChatEvent::Exited { code, aborted });
        });
        Ok(())
    }

    fn write_stdin(&self, session_id: &str, text: &str) -> Result<(), AppError> {
        let mut running = self.shared.running.lock().unwrap();
        let entry = running
            .get_mut(session_id)
            .ok_or_else(|| AppError::Process("세션 프로세스가 없습니다.".into()))?;
        entry
            .stdin
            .send(format!("{}\n", encode_user_message(text)))
            .map_err(|_| AppError::Process("세션 프로세스가 이미 종료되었습니다.".into()))?;
        entry.busy = true;
        Ok(())
    }

    fn respond_permission(
        &self,
        session_id: &str,
        request_id: &str,
        allow: bool,
        updated_permissions: Option<&str>,
    ) -> Result<(), AppError> {
        let running = self.shared.running.lock().unwrap();
        let entry = running
            .get(session_id)
            .ok_or_else(|| AppError::Process("세션 프로세스가 없습니다.".into()))?;
        entry
            .stdin
            .send(format!("{}\n", encode_permission_response(request_id, allow, updated_permissions)))
            .map_err(|_| AppError::Process("세션 프로세스가 이미 종료되었습니다.".into()))
    }

    fn kill(&self, session_id: &str) -> Result<(), AppError> {
        let removed = self.shared.running.lock().unwrap().remove(session_id);
        if let Some(entry) = removed {
            self.shared.mark_stopped(session_id);
            self.shared.aborted.lock().unwrap().push(entry.generation);
            if let Some(pid) = entry.pid {
                kill_tree(pid);
            }
        }
        Ok(())
    }

    fn kill_all(&self) {
        let pids: Vec<u32> =
            self.shared.running.lock().unwrap().drain().filter_map(|(_, entry)| entry.pid).collect();
        for pid in pids {
            kill_tree(pid);
        }
    }

    fn status(&self, session_id: &str) -> Option<ProcessStatus> {
        self.shared
            .running
            .lock()
            .unwrap()
            .get(session_id)
            .map(|entry| ProcessStatus {
                busy: entry.busy,
                model: entry.model.clone(),
                permission_mode: entry.permission_mode.clone(),
            })
    }

    fn stopped_at(&self, session_id: &str) -> Option<f64> {
        self.shared.stopped_at.lock().unwrap().get(session_id).copied()
    }

    fn subscribe(&self) -> Receiver<(String, ChatEvent)> {
        let (tx, rx) = channel();
        self.shared.subscribers.lock().unwrap().push(tx);
        rx
    }
}
