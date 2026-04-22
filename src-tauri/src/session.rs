use anyhow::Result;
use portable_pty::{native_pty_system, CommandBuilder, PtySize};
use std::io::{Read, Write};
use tauri::Emitter;

pub struct Session {
    pub writer: Box<dyn Write + Send>,
    pub master: Box<dyn portable_pty::MasterPty + Send>,
}

impl Session {
    pub fn new(
        session_id: String,
        path: std::path::PathBuf,
        width: u16,
        height: u16,
        app_handle: tauri::AppHandle,
    ) -> Result<Self> {
        let pty_system = native_pty_system();
        let pair = pty_system.openpty(PtySize {
            rows: height,
            cols: width,
            pixel_width: 0,
            pixel_height: 0,
        })?;

        #[cfg(target_os = "windows")]
        let shell = std::env::var("COMSPEC").unwrap_or_else(|_| "cmd.exe".to_string());
        #[cfg(not(target_os = "windows"))]
        let shell = std::env::var("SHELL").unwrap_or_else(|_| "sh".to_string());

        let mut cmd = CommandBuilder::new(shell);
        cmd.cwd(path);

        let _child = pair.slave.spawn_command(cmd)?;

        let mut reader = pair.master.try_clone_reader()?;
        let writer = pair.master.take_writer()?;
        let master = pair.master;

        let session_id_clone = session_id.clone();
        let app_handle_clone = app_handle.clone();
        std::thread::spawn(move || {
            let mut buf = [0u8; 4096];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) => {
                        let _ = app_handle_clone.emit(
                            "pty-exit",
                            serde_json::json!({ "session_id": session_id_clone }),
                        );
                        break;
                    }
                    Ok(n) => {
                        let _ = app_handle_clone.emit(
                            "pty-output",
                            serde_json::json!({
                                "session_id": session_id_clone,
                                "data": &buf[..n]
                            }),
                        );
                    }
                    Err(_) => break,
                }
            }
        });

        Ok(Self { writer, master })
    }

    pub fn write(&mut self, data: &[u8]) -> Result<()> {
        self.writer.write_all(data)?;
        self.writer.flush()?;
        Ok(())
    }

    pub fn resize(&mut self, width: u16, height: u16) -> Result<()> {
        self.master.resize(PtySize {
            rows: height,
            cols: width,
            pixel_width: 0,
            pixel_height: 0,
        })?;
        Ok(())
    }
}
