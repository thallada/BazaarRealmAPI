use refinery::config::Config;

mod migrations;

pub async fn migrate() {
    let mut config = Config::from_file_location("src/db/refinery.toml").unwrap();

    match migrations::runner().run_async(&mut config).await {
        Ok(report) => {
            dbg!(report.applied_migrations());
        }
        Err(error) => {
            dbg!(error);
        }
    };
}
