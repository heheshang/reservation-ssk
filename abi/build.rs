use std::process::Command;

use tonic_build::Builder;

fn main() {
    tonic_build::configure()
        .out_dir("src/pb")
        .with_sql_type(&["reservation.ReservationStatus"])
        .with_builder(&[
            "reservation.ReservationQuery",
            "reservation.ReservationFilter",
        ])
        .with_builder_into(
            "reservation.ReservationQuery",
            &[
                "resource_id",
                "user_id",
                "status",
                "page",
                "page_size",
                "desc",
            ],
        )
        .with_builder_into(
            "reservation.ReservationFilter",
            &[
                "resource_id",
                "user_id",
                "status",
                "cursor",
                "page_size",
                "desc",
            ],
        )
        .with_builder_option("reservation.ReservationQuery", &["start", "end"])
        .compile(&["protos/reservation.proto"], &["protos"])
        .unwrap();
    // fs::remove_file("path/to/file").unwrap();

    Command::new("cargo").args(["fmt"]).output().unwrap();

    println!("cargo:rerun-if-changed=protos/reservation.proto");
}

trait BuilderExt {
    fn with_sql_type(self, paths: &[&str]) -> Self;
    fn with_builder(self, paths: &[&str]) -> Self;
    fn with_builder_into(self, paths: &str, fields: &[&str]) -> Self;
    fn with_builder_option(self, paths: &str, fields: &[&str]) -> Self;
}
impl BuilderExt for Builder {
    fn with_sql_type(self, paths: &[&str]) -> Self {
        paths
            .iter()
            .fold(self, |b, p| b.type_attribute(p, "#[derive(sqlx::Type)]"))
    }
    fn with_builder(self, paths: &[&str]) -> Self {
        paths.iter().fold(self, |b, p| {
            b.type_attribute(p, "#[derive(derive_builder::Builder)]")
        })
    }
    fn with_builder_into(self, paths: &str, fields: &[&str]) -> Self {
        fields.iter().fold(self, |b, f| {
            b.field_attribute(
                format!("{}.{}", paths, f),
                "#[builder(setter(into),default)]",
            )
        })
    }
    fn with_builder_option(self, paths: &str, fields: &[&str]) -> Self {
        fields.iter().fold(self, |b, f| {
            b.field_attribute(
                format!("{}.{}", paths, f),
                "#[builder(setter(into,strip_option),default)]",
            )
        })
    }
}
