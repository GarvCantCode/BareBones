mod launcher;

fn main() {
    let launch = launcher::Launcher::new();
    launch.start();
}