use mev_rs::run;

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();

    run().await;
}
