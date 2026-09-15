use std::env;
use std::process;

fn main() {
    let args: Vec<String> = env::args().collect();
    let is_check = args.iter().any(|a| a == "--check");

    let path = diskclearance_lib::boundary::generator::default_bindings_path();

    if is_check {
        match diskclearance_lib::boundary::generator::check_bindings(&path) {
            Ok(true) => {
                println!("TypeScript bindings are up to date at {}", path.display());
            }
            Ok(false) => {
                eprintln!(
                    "ERROR: TypeScript bindings at {} are stale!\nRun 'npm run types:generate' to update them.",
                    path.display()
                );
                process::exit(1);
            }
            Err(err) => {
                eprintln!("ERROR: Failed checking bindings: {err}");
                process::exit(1);
            }
        }
    } else {
        if let Err(err) = diskclearance_lib::boundary::generator::write_bindings(&path) {
            eprintln!(
                "ERROR: Failed writing bindings to {}: {err}",
                path.display()
            );
            process::exit(1);
        }
        println!(
            "Successfully generated TypeScript bindings at {}",
            path.display()
        );
    }
}
