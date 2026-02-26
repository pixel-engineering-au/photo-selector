pub mod image_index;
pub mod navigation;
pub mod app_state;
pub mod file_ops;
pub mod image_cache;
pub mod events;
pub mod undo;
pub mod stats;

pub fn hello() -> &'static str {
    "Hello from photo-selector core"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hello_works() {
        assert_eq!(hello(), "Hello from photo-selector core");
    }
}