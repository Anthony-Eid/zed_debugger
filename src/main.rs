use std::process;

fn main() {
    let pid = process::id();
    println!("Current process ID: {}", pid);
    let x = 10;

    random_func();
    dbg!(x);

    let y = 30;

    println!("Hello world!!");

    loop {
        println!("Hello world: {}", pid);
    }
}

fn random_func() {
    let y = 20;

    let x = 30;
}
