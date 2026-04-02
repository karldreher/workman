use anyhow::Result;
use portable_pty::{native_pty_system, CommandBuilder, PtySize};
use std::sync::{Arc, Mutex};
use std::io::{Read, Write};
use vt100::Parser; // Removed Screen import here

pub struct Session {
    pub parser: Arc<Mutex<Parser>>,
    pub writer: Box<dyn Write + Send>,
    pub master: Box<dyn portable_pty::MasterPty + Send>,
}

impl Session {
    pub fn new(path: std::path::PathBuf, width: u16, height: u16) -> Result<Self> {
        let pty_system = native_pty_system();
        let pair = pty_system.openpty(PtySize {
            rows: height,
            cols: width,
            pixel_width: 0,
            pixel_height: 0,
        })?;

        let shell = std::env::var("SHELL").unwrap_or_else(|_| "sh".to_string());
        let mut cmd = CommandBuilder::new(shell);
        cmd.cwd(path);

        let mut _child = pair.slave.spawn_command(cmd)?;
        
        let parser = Arc::new(Mutex::new(Parser::new(height, width, 1000)));
        let parser_clone = parser.clone();
        
        let mut reader = pair.master.try_clone_reader()?;
        let writer = pair.master.take_writer()?;
        let master = pair.master;

        tokio::task::spawn_blocking(move || {
            let mut buf = [0u8; 4096];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => {
                        let mut p = parser_clone.lock().unwrap();
                        p.process(&buf[..n]);
                    }
                    Err(_) => break,
                }
            }
        });

        Ok(Self {
            parser,
            writer,
            master,
        })
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
        self.parser.lock().unwrap().set_size(height, width);
        Ok(())
    }
}

