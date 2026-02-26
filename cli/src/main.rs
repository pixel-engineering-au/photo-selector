use photo_selector_core::app_state::{AppState, Action};
use photo_selector_core::events::AppEvent;
use photo_selector_core::image_index::SortOrder;
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
                let size = img.file_size
                    .map(|b| format!(" ({}KB)", b / 1024))
                    .unwrap_or_default();
                println!("{}: {}{}", i + 1, name, size);
            }
        }

        let undo_hint = if app.can_undo() { " | [u] undo" } else { "" };
        println!(
            "\n[n] next | [p] prev | [s <n>] select | [r <n>] reject{} | [sort <order>] | [view <n>] | [q] quit",
            undo_hint
        );
        println!("  sort orders: name-asc, name-desc, date-asc, date-desc, size-asc, size-desc");
        print!("> ");
        io::stdout().flush().unwrap();

        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        let parts: Vec<&str> = input.trim().split_whitespace().collect();

        match parts.as_slice() {
            ["n"] => print_events(&app.next()),
            ["p"] => print_events(&app.prev()),
            ["u"] => print_events(&app.undo().unwrap()),
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
            ["sort", order] => {
                let sort = match *order {
                    "name-asc"   => Some(SortOrder::NameAsc),
                    "name-desc"  => Some(SortOrder::NameDesc),
                    "date-asc"   => Some(SortOrder::DateModifiedAsc),
                    "date-desc"  => Some(SortOrder::DateModifiedDesc),
                    "size-asc"   => Some(SortOrder::SizeAsc),
                    "size-desc"  => Some(SortOrder::SizeDesc),
                    _ => { println!("Unknown sort order"); None }
                };
                if let Some(s) = sort {
                    let e = app.set_sort_order(s);
                    if e.is_empty() {
                        println!("Already using that sort order.");
                    } else {
                        print_events(&e);
                    }
                }
            }
            ["view", n] => {
                if let Ok(count) = n.parse::<usize>() {
                    if count == 0 {
                        println!("view count must be > 0");
                    } else {
                        print_events(&app.set_view_count(count));
                    }
                } else {
                    println!("Invalid number");
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
            AppEvent::FileMoved { from, to, action } => {
                let verb = match action {
                    photo_selector_core::events::MoveAction::Select => "Selected",
                    photo_selector_core::events::MoveAction::Reject => "Rejected",
                };
                println!(
                    "{}: {} -> {}",
                    verb,
                    from.file_name().unwrap_or_default().to_string_lossy(),
                    to.display()
                );
            }
            AppEvent::Undone { path, action } => {
                let verb = match action {
                    photo_selector_core::events::MoveAction::Select => "select",
                    photo_selector_core::events::MoveAction::Reject => "reject",
                };
                println!(
                    "Undid {}: {} restored",
                    verb,
                    path.file_name().unwrap_or_default().to_string_lossy()
                );
            }
            AppEvent::UndoStackEmpty => {
                println!("Nothing to undo.");
            }
            AppEvent::StatsChanged(stats) => {
                println!(
                    "Progress: {}% — {} remaining, {} selected, {} rejected",
                    stats.progress_percent(),
                    stats.remaining,
                    stats.selected,
                    stats.rejected,
                );
            }
            AppEvent::SortChanged { order } => {
                println!("Sort order changed to: {:?}", order);
            }
            AppEvent::ViewCountChanged { view_count } => {
                println!("Now showing {} image(s) per page.", view_count);
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