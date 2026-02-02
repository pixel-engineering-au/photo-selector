use photo_selector_core::app_state::{AppState, Action};
use std::env;
use std::io::{self, Write};
use std::path::PathBuf;

fn main() {
    let dir = env::args()
        .nth(1)
        .expect("Usage: photo_selector_cli <photo_dir>");

    let mut app = AppState::new(4);
    app.load_dir(&PathBuf::from(dir));

    loop {
        println!("\n=== Current Images ===");

        let images = app.current_images();
        if images.is_empty() {
            println!("(no images)");
        } else {
            for (i, img) in images.iter().enumerate() {
                let name = img
                    .path
                    .file_name()
                    .unwrap()
                    .to_string_lossy();

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
            ["n"] => app.next(),
            ["p"] => app.prev(),
            ["s", idx] => {
                if let Ok(i) = idx.parse::<usize>() {
                    app.act_on_current_at(Action::Select, i.saturating_sub(1)).unwrap();
                } else {
                    println!("Invalid index");
                }
            }
            ["r", idx] => {
                if let Ok(i) = idx.parse::<usize>() {
                    app.act_on_current_at(Action::Reject, i.saturating_sub(1)).unwrap();
                } else {
                    println!("Invalid index");
                }
            }
            ["q"] => break,
            _ => println!("Unknown command"),
        }


    }
}
