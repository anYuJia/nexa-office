use std::{
    ffi::OsStr,
    io,
    path::{Path, PathBuf},
    process::{Command, Output},
};

const OFFICE_FILTER: &str = "*.docx *.xlsx *.pptx";

pub fn choose_open_file() -> io::Result<Option<PathBuf>> {
    #[cfg(target_os = "macos")]
    {
        return run_path_command(
            Command::new("osascript")
                .arg("-e")
                .arg("POSIX path of (choose file with prompt \"Open Office document\")"),
        );
    }

    #[cfg(target_os = "windows")]
    {
        let script = concat!(
            "Add-Type -AssemblyName System.Windows.Forms;",
            "$d=New-Object System.Windows.Forms.OpenFileDialog;",
            "$d.Filter='Office documents (*.docx;*.xlsx;*.pptx)|*.docx;*.xlsx;*.pptx';",
            "if($d.ShowDialog() -eq 'OK'){[Console]::Write($d.FileName)}"
        );
        return run_path_command(
            Command::new("powershell")
                .args(["-NoProfile", "-STA", "-Command", script]),
        );
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    {
        return run_path_command(
            Command::new("zenity")
                .args(["--file-selection", "--title=Open Office document"])
                .arg(format!("--file-filter=Office documents | {OFFICE_FILTER}")),
        );
    }

    #[allow(unreachable_code)]
    Ok(None)
}

pub fn choose_save_file(extension: &str, suggested_name: &str) -> io::Result<Option<PathBuf>> {
    let extension = extension.trim_start_matches('.');
    #[cfg(target_os = "macos")]
    {
        let script = format!(
            "POSIX path of (choose file name with prompt \"Save Office document\" default name \"{}\")",
            applescript_escape(suggested_name)
        );
        return run_path_command(Command::new("osascript").arg("-e").arg(script))
            .map(|path| path.map(|path| ensure_extension(path, extension)));
    }

    #[cfg(target_os = "windows")]
    {
        let escaped = powershell_escape(suggested_name);
        let script = format!(
            "Add-Type -AssemblyName System.Windows.Forms;             $d=New-Object System.Windows.Forms.SaveFileDialog;             $d.Filter='Office document (*.{extension})|*.{extension}';             $d.FileName='{escaped}';             if($d.ShowDialog() -eq 'OK'){{[Console]::Write($d.FileName)}}"
        );
        return run_path_command(
            Command::new("powershell")
                .args(["-NoProfile", "-STA", "-Command", &script]),
        )
        .map(|path| path.map(|path| ensure_extension(path, extension)));
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    {
        return run_path_command(
            Command::new("zenity")
                .args([
                    "--file-selection",
                    "--save",
                    "--confirm-overwrite",
                    "--title=Save Office document",
                ])
                .arg(format!("--filename={suggested_name}"))
                .arg(format!("--file-filter=Office document | *.{extension}")),
        )
        .map(|path| path.map(|path| ensure_extension(path, extension)));
    }

    #[allow(unreachable_code)]
    Ok(None)
}

pub fn print_file(path: &Path) -> io::Result<()> {
    #[cfg(target_os = "macos")]
    let status = Command::new("lp").arg(path).status()?;

    #[cfg(target_os = "windows")]
    let status = {
        let quoted = powershell_escape(&path.to_string_lossy());
        Command::new("powershell")
            .args([
                "-NoProfile",
                "-Command",
                &format!("Start-Process -FilePath '{quoted}' -Verb Print"),
            ])
            .status()?
    };

    #[cfg(all(unix, not(target_os = "macos")))]
    let status = Command::new("lp").arg(path).status()?;

    #[cfg(not(any(unix, target_os = "windows")))]
    return Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "printing is not supported on this platform",
    ));

    if status.success() {
        Ok(())
    } else {
        Err(io::Error::other("native print command failed"))
    }
}

pub fn copy_text(text: &str) -> io::Result<()> {
    #[cfg(target_os = "macos")]
    {
        return pipe_text(Command::new("pbcopy"), text);
    }

    #[cfg(target_os = "windows")]
    {
        let escaped = powershell_escape(text);
        return command_success(
            Command::new("powershell")
                .args(["-NoProfile", "-Command", &format!("Set-Clipboard -Value '{escaped}'")]),
        );
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    {
        if pipe_text(Command::new("wl-copy"), text).is_ok() {
            return Ok(());
        }
        return pipe_text(Command::new("xclip").args(["-selection", "clipboard"]), text);
    }

    #[allow(unreachable_code)]
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "clipboard is not supported on this platform",
    ))
}

fn ensure_extension(mut path: PathBuf, extension: &str) -> PathBuf {
    let matches = path
        .extension()
        .and_then(OsStr::to_str)
        .is_some_and(|value| value.eq_ignore_ascii_case(extension));
    if !matches {
        path.set_extension(extension);
    }
    path
}

fn run_path_command(command: &mut Command) -> io::Result<Option<PathBuf>> {
    let output = command.output()?;
    parse_path_output(output)
}

fn parse_path_output(output: Output) -> io::Result<Option<PathBuf>> {
    if !output.status.success() {
        return Ok(None);
    }
    let text = String::from_utf8_lossy(&output.stdout);
    let value = text.trim();
    if value.is_empty() {
        Ok(None)
    } else {
        Ok(Some(PathBuf::from(value)))
    }
}

fn pipe_text(command: &mut Command, text: &str) -> io::Result<()> {
    use std::io::Write;
    use std::process::Stdio;

    let mut child = command.stdin(Stdio::piped()).spawn()?;
    if let Some(stdin) = child.stdin.as_mut() {
        stdin.write_all(text.as_bytes())?;
    }
    let status = child.wait()?;
    if status.success() {
        Ok(())
    } else {
        Err(io::Error::other("clipboard command failed"))
    }
}

fn command_success(command: &mut Command) -> io::Result<()> {
    let status = command.status()?;
    if status.success() {
        Ok(())
    } else {
        Err(io::Error::other("native command failed"))
    }
}

fn powershell_escape(value: &str) -> String {
    value.replace('\'', "''")
}

fn applescript_escape(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn save_path_gets_required_extension() {
        assert_eq!(
            ensure_extension(PathBuf::from("report"), "docx"),
            PathBuf::from("report.docx")
        );
        assert_eq!(
            ensure_extension(PathBuf::from("report.DOCX"), "docx"),
            PathBuf::from("report.DOCX")
        );
    }

    #[test]
    fn shell_escaping_is_bounded() {
        assert_eq!(powershell_escape("a'b"), "a''b");
        assert_eq!(applescript_escape("a\"b"), "a\\\"b");
    }
}
