use std::process::Command;

macro_rules! get_command_output {
    ($cmd:expr, $( $arg:expr ),* ) => {
        {
            let output = Command::new($cmd)
                $( .arg($arg) )*
                .output()
                .expect("Failed to execute command");
            String::from_utf8(output.stdout).expect("Failed to convert output to string")
        }
    };
}

fn main() {
    embuild::espidf::sysenv::output();

    let current_date_time = get_command_output!("date", "+%d.%m.%Y %H:%M:%S");
    let git_hash = get_command_output!("git", "rev-parse", "--short", "HEAD");
    let git_is_tagged = Command::new("git")
        .arg("describe")
        .arg("--exact-match")
        .arg("HEAD")
        .output()
        .unwrap()
        .status
        .success();

    println!(
        "cargo:rustc-env=PWOS_REL_OR_DEV=-{}",
        if git_is_tagged { "release" } else { "devel" }
    );
    println!("cargo:rustc-env=PWOS_COMMIT={git_hash}");
    println!("cargo:rustc-env=BUILD_DATE_TIME={current_date_time}");
}
