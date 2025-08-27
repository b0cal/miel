use miel::*;
use controller_handler::*;

fn main() {
    let _controller = Controller::new();
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}