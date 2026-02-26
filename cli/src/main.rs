use photo_selector_core::app_state::{AppState, Action};
use photo_selector_core::events::AppEvent;
use std::env;
use std::io::{self, Write};
use std::path::PathBuf;

fn main() {
    let dir = env::args()
        .nth(1)
        .expect("Usage: photo_selector_cli <photo_dir>");

    let mut app = AppState::new(4);

    let events = app.load_dir(&PathBuf::from(dir));
    print_events(&events);

    loop {
        println!("\n=== Current Images ===");

        let (images, events) = app.current_images();
        print_events(&events);

        if images.is_empty() {
            println!("(no images)");
        } else {
            for (i, img) in images.iter().enumerate() {
                let name = img.path.file_name().unwrap().to_string_lossy();
                println!("{}: {}", i + 1, name);
            }
        }

        println!("\n[n] next | [p] prev | [s <n>] select | [r <n>] reject | [q] quit");
        print!("> ");
        io::stdout().flush().unwrap();

        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();

        let parts: Vec<&str> = input.trim().split_whitespace().collect();

        match parts.as_slice() {
            ["n"] => { let e = app.next();  print_events(&e); }
            ["p"] => { let e = app.prev();  print_events(&e); }
            ["s", idx] => {
                if let Ok(i) = idx.parse::<usize>() {
                    let e = app.act_on_current_at(Action::Select, i.saturating_sub(1)).unwrap();
                    print_events(&e);
                } else {
                    println!("Invalid index");
                }
            }
            ["r", idx] => {
                if let Ok(i) = idx.parse::<usize>() {
                    let e = app.act_on_current_at(Action::Reject, i.saturating_sub(1)).unwrap();
                    print_events(&e);
                } else {
                    println!("Invalid index");
                }
            }
            ["q"] => break,
            _ => println!("Unknown command"),
        }
    }
}

/// Translates core events into human-readable CLI output.
/// This is the only place in the CLI that knows about AppEvent variants.
fn print_events(events: &[AppEvent]) {
    for event in events {
        match event {
            AppEvent::DirectoryLoaded { path, total } => {
                println!("Loaded: {} ({} images)", path.display(), total);
            }
            AppEvent::FileMoved { from, to, .. } => {
                println!(
                    "Moved: {} -> {}",
                    from.file_name().unwrap_or_default().to_string_lossy(),
                    to.display()
                );
            }
            AppEvent::StaleEntryRemoved { path } => {
                println!(
                    "Removed stale entry: {}",
                    path.file_name().unwrap_or_default().to_string_lossy()
                );
            }
            AppEvent::LibraryEmpty => {
                println!("No more images in library.");
            }
            AppEvent::NavigationBoundary { kind } => {
                use photo_selector_core::events::BoundaryKind;
                match kind {
                    BoundaryKind::FirstPage => println!("Already at first page."),
                    BoundaryKind::LastPage  => println!("Already at last page."),
                }
            }
            // PageChanged is for GUI consumers — CLI re-renders on next loop iteration
            AppEvent::PageChanged(_) => {}
        }
    }
}