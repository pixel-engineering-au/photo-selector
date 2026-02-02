pub mod image_index;

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
