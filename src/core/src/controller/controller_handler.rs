pub struct Controller {
    // Fields for the Controller struct
    pub foo: i32,
}

// Constructor synatx
impl Controller {
    pub fn new() -> Controller {
        Controller { foo: 0 }
    }
}

fn private_function() {
    // This function is private to the module
}

pub mod nested {
    pub fn nested_function() {
        // This function is public within the nested module
    }
}

impl Controller {}
