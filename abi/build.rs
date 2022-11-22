use std::process::Command;

fn main() {
    tonic_build::configure()
        .out_dir("src/pb")
        .compile(&["protos/reservation.proto"], &["protos"])
        .unwrap();
    // fs::remove_file("path/to/file").unwrap();

    Command::new("cargo").args(&["fmt"]).output().unwrap();

    println!("cargo:rerun-if-changed=protos/reservation.proto");

    // tonic_build::configure()
    //     .out_dir("src/pb")
    //     .compile(&["protos/reservation.proto"], &["protos"])
    //     .unwrap();

    // Command::new("cargo").args(&["fmt"]).output().unwrap();

    // println!("cargo:rerun-if-changed=protos/reservation.proto");
}
