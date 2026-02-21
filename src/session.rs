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

#[cfg(test)]
mod tests {
    // use super::*; // Removed unused import: Screen
    // use std::thread; // Keep if needed for other tests
    // // Removed unused import: use tempfile::tempdir;

    // #[tokio::test]
    // async fn test_session_creation_and_write() -> Result<()> {
    //     thread::sleep(std::time::Duration::from_secs(3)); // Give PTY system time to initialize

    //     // Using home_dir for a stable environment
    //     let path = dirs::home_dir().unwrap();

    //     // Simulate a small terminal size
    //     let width = 80;
    //     let height = 24;

    //     let mut session = Session::new(path.clone(), width, height)?;

    //     // Give some time for the shell to start and print its prompt
    //     tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;

    //     // Write a command and a newline
    //     session.write(b"echo Hello, World!\r\n")?; 
        
    //     // Give some time for the command to execute and output
    //     tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;

    //     let parser = session.parser.lock().unwrap();
    //     let screen = parser.screen();
        
    //     let mut full_content = String::new();
    //     let (screen_height, screen_width) = screen.size();
    //     for row in 0..screen_height {
    //         for col in 0..screen_width {
    //             if let Some(cell) = screen.cell(row, col) {
    //                 full_content.push_str(&cell.contents());
    //             }
    //         }
    //         full_content.push('\n');
    //     }
        
    //     assert!(full_content.contains("Hello, World!"), "Expected 'Hello, World!' in session output. Screen content:\n{}", full_content);

    //     // Clear for next check by re-initializing the parser
    //     drop(parser); // Release lock
    //     session.parser = Arc::new(Mutex::new(Parser::new(height, width, 1000)));
        
    //     session.write(b"ls -l\r\n")?;
    //     tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;

    //     let parser = session.parser.lock().unwrap();
    //     let screen = parser.screen();
        
    //     let mut full_content = String::new();
    //     let (screen_height, screen_width) = screen.size();
    //     for row in 0..screen_height {
    //         for col in 0..screen_width {
    //             if let Some(cell) = screen.cell(row, col) {
    //                 full_content.push_str(&cell.contents());
    //             }
    //         }
    //         full_content.push('\n');
    //     }
    //     assert!(!full_content.trim().is_empty(), "Expected non-empty output in session output. Screen content:\n{}", full_content);

    //     Ok(())
    // }
}
