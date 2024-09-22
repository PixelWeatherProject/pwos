use std::process::Command;

fn main() {
    embuild::espidf::sysenv::output();

    let git_hash_raw = Command::new("git")
        .arg("rev-parse")
        .arg("--short")
        .arg("HEAD")
        .output()
        .unwrap()
        .stdout;
    let git_hash = unsafe { String::from_utf8_unchecked(git_hash_raw) };
    println!("cargo:rustc-env=PWOS_COMMIT={git_hash}");
}
