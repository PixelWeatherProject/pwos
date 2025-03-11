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
    println!(
        "cargo:rustc-env=BUILD_DATE_TIME={}",
        get_current_date_time()
    );
}

fn get_current_date_time() -> String {
    String::from_utf8(
        Command::new("date")
            .arg("+%d.%m.%Y %H:%M:%S")
            .output()
            .unwrap()
            .stdout,
    )
    .unwrap()
}
