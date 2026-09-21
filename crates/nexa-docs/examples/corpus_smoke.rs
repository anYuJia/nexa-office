use nexa_docs::{Paginator, open_docx, save_docx};
use std::{fs::File, io::Cursor, path::Path};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let paths: Vec<String> = std::env::args().skip(1).collect();
    if paths.is_empty() {
        return Err("provide one or more DOCX files".into());
    }

    for path in paths {
        inspect(Path::new(&path))?;
    }

    Ok(())
}

fn inspect(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let file = File::open(path)?;
    let mut document = open_docx(file)?;
    let layout = Paginator::default().layout(document.document());
    let issues = document.document().compatibility.issues.len();
    let can_save = document.document().compatibility.can_save();

    println!(
        "file={} paragraphs={} pages={} compatibility_issues={} can_save={}",
        path.display(),
        document.document().paragraph_count(),
        layout.page_count(),
        issues,
        can_save
    );

    if can_save {
        let bytes = save_docx(&mut document, Cursor::new(Vec::new()))?.into_inner();
        let reopened = open_docx(Cursor::new(bytes))?;
        if reopened.document().paragraph_count() == 0 {
            return Err(format!("{} reopened with no paragraphs", path.display()).into());
        }
    }

    Ok(())
}
