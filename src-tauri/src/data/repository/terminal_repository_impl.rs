use std::collections::HashMap;
use std::io::{Read, Write};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};

use portable_pty::{native_pty_system, ChildKiller, CommandBuilder, MasterPty, PtySize};

use crate::domain::entity::{AppError, TerminalKind};
use crate::domain::repository::{TerminalOutput, TerminalRepository, TerminalSpec};
use crate::infrastructure::platform::TerminalDefaults;
use crate::infrastructure::process::kill_tree;

struct Term {
    master: Box<dyn MasterPty + Send>,
    writer: Box<dyn Write + Send>,
    killer: Box<dyn ChildKiller + Send + Sync>,
    pid: Option<u32>,
}

#[derive(Default)]
struct Shared {
    terms: Mutex<HashMap<String, Term>>,
    subscribers: Mutex<Vec<Sender<TerminalOutput>>>,
}

impl Shared {
    fn publish(&self, output: TerminalOutput) {
        self.subscribers.lock().unwrap().retain(|tx| tx.send(output.clone()).is_ok());
    }
}

pub struct TerminalRepositoryImpl {
    defaults: TerminalDefaults,
    is_windows: bool,
    shared: Arc<Shared>,
}

impl TerminalRepositoryImpl {
    pub fn new(defaults: TerminalDefaults, is_windows: bool) -> Self {
        Self { defaults, is_windows, shared: Arc::default() }
    }
}

fn io_err(e: impl std::fmt::Display) -> AppError {
    AppError::Process(e.to_string())
}

impl TerminalRepository for TerminalRepositoryImpl {
    fn default_path(&self, kind: TerminalKind) -> Option<String> {
        let path = match kind {
            TerminalKind::Cmd => &self.defaults.cmd,
            TerminalKind::PowerShell => &self.defaults.power_shell,
            TerminalKind::GitBash => &self.defaults.git_bash,
            TerminalKind::MacTerminal => &self.defaults.mac_terminal,
        };
        path.as_ref().map(|p| p.to_string_lossy().into_owned())
    }

    fn is_windows(&self) -> bool {
        self.is_windows
    }

    fn spawn(&self, spec: TerminalSpec) -> Result<(), AppError> {
        if self.shared.terms.lock().unwrap().contains_key(&spec.id) {
            return Err(AppError::Process(format!("이미 쓰고 있는 터미널 id 입니다: {}", spec.id)));
        }
        let pair = native_pty_system()
            .openpty(PtySize { rows: spec.rows.max(5), cols: spec.cols.max(20), pixel_width: 0, pixel_height: 0 })
            .map_err(io_err)?;
        let mut command = CommandBuilder::new(&spec.program);
        command.args(&spec.args);
        command.cwd(&spec.cwd);
        command.env("TERM", "xterm-256color");
        let mut child = pair
            .slave
            .spawn_command(command)
            .map_err(|e| AppError::Process(format!("{} 실행 실패: {e}", spec.program)))?;
        drop(pair.slave);

        let id = spec.id;
        let mut reader = pair.master.try_clone_reader().map_err(io_err)?;
        let writer = pair.master.take_writer().map_err(io_err)?;
        let term = Term { master: pair.master, writer, killer: child.clone_killer(), pid: child.process_id() };
        self.shared.terms.lock().unwrap().insert(id.clone(), term);

        let (shared, term_id) = (self.shared.clone(), id.clone());
        std::thread::spawn(move || {
            let mut buf = [0u8; 8192];
            let mut pending = Vec::new();
            while let Ok(n) = reader.read(&mut buf) {
                if n == 0 {
                    break;
                }
                pending.extend_from_slice(&buf[..n]);
                let data = take_utf8(&mut pending);
                if !data.is_empty() {
                    shared.publish(TerminalOutput::Data { id: term_id.clone(), data });
                }
            }
        });

        // Windows ConPTY 는 자식이 끝나도 읽기가 EOF 가 되지 않으므로 종료는 따로 기다린다.
        let (shared, term_id) = (self.shared.clone(), id.clone());
        std::thread::spawn(move || {
            let code = child.wait().ok().map(|status| status.exit_code());
            shared.publish(TerminalOutput::Exit { id: term_id.clone(), code });
            // PTY 를 닫아 읽기 스레드도 끝낸다.
            shared.terms.lock().unwrap().remove(&term_id);
        });
        Ok(())
    }

    fn write(&self, id: &str, data: &str) -> Result<(), AppError> {
        let mut terms = self.shared.terms.lock().unwrap();
        let term = terms.get_mut(id).ok_or_else(|| AppError::NotFound("터미널이 이미 종료되었습니다.".into()))?;
        term.writer.write_all(data.as_bytes()).and_then(|_| term.writer.flush()).map_err(io_err)
    }

    fn resize(&self, id: &str, cols: u16, rows: u16) -> Result<(), AppError> {
        let terms = self.shared.terms.lock().unwrap();
        if let Some(term) = terms.get(id) {
            term.master
                .resize(PtySize { rows: rows.max(5), cols: cols.max(20), pixel_width: 0, pixel_height: 0 })
                .map_err(io_err)?;
        }
        Ok(())
    }

    fn kill(&self, id: &str) {
        let removed = self.shared.terms.lock().unwrap().remove(id);
        if let Some(mut term) = removed {
            // 셸이 띄운 프로그램까지 끝낸다.
            if let Some(pid) = term.pid {
                kill_tree(pid);
            }
            let _ = term.killer.kill();
        }
    }

    fn kill_all(&self) {
        let ids: Vec<String> = self.shared.terms.lock().unwrap().keys().cloned().collect();
        for id in ids {
            self.kill(&id);
        }
    }

    fn subscribe(&self) -> Receiver<TerminalOutput> {
        let (tx, rx) = channel();
        self.shared.subscribers.lock().unwrap().push(tx);
        rx
    }
}

/// 버퍼에서 완성된 UTF-8 앞부분만 꺼낸다. 읽기 경계에서 잘린 멀티바이트 문자는 다음 조각과 합친다.
fn take_utf8(pending: &mut Vec<u8>) -> String {
    let valid = match std::str::from_utf8(pending) {
        Ok(_) => pending.len(),
        // 끝이 잘린 문자면 그 앞까지, 중간에 깨진 바이트면 대체 문자로 넘긴다.
        Err(e) if e.error_len().is_none() => e.valid_up_to(),
        Err(_) => pending.len(),
    };
    let rest = pending.split_off(valid);
    let text = String::from_utf8_lossy(pending).into_owned();
    *pending = rest;
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn multibyte_characters_split_across_reads_are_joined() {
        let bytes = "가나".as_bytes();
        let mut pending = bytes[..4].to_vec();
        assert_eq!(take_utf8(&mut pending), "가");
        assert_eq!(pending.len(), 1);
        pending.extend_from_slice(&bytes[4..]);
        assert_eq!(take_utf8(&mut pending), "나");
        assert!(pending.is_empty());
    }

    #[test]
    fn broken_bytes_do_not_stall_the_stream() {
        let mut pending = vec![b'a', 0xff, b'b'];
        assert_eq!(take_utf8(&mut pending), "a\u{fffd}b");
        assert!(pending.is_empty());
    }
}
