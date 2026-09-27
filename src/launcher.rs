pub struct Launcher {
    name: String,
}

impl Launcher {
    pub fn start(&self) { 
        println!("Starting {}", self.name);
    }

    pub fn new() -> Launcher {
        Launcher { name: "BareBones".to_string()}
    }
}