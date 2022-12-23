use sqlx::postgres::PgListener;
use tokio_stream::StreamExt;
#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    let url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let mut listener = PgListener::connect(&url).await.unwrap();
    listener.listen("reservation_update").await.unwrap();
    println!("listening for reservation_update notifications");

    let stream = listener.into_stream();
    let stream = stream.throttle(std::time::Duration::from_secs(10));
    tokio::pin!(stream);

    while let Some(Ok(event)) = stream.next().await {
        println!("received event: {:?}", event);
    }
}
