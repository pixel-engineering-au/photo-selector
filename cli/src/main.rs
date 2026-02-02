use photo_selector_core::app_state::AppState;
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
                println!("{}: {}", i + 1, img.filename);
            }
        }

        println!("\n[n] next | [p] prev | [q] quit");
        print!("> ");
        io::stdout().flush().unwrap();

        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();

        match input.trim() {
            "n" => app.next(),
            "p" => app.prev(),
            "q" => break,
            _ => println!("Unknown command"),
        }
    }
}
