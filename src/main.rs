use hyper::{body::HttpBody, client::HttpConnector, Client, Uri};
use num_format::{Locale, ToFormattedString};
use rocket::{figment::Figment, log::LogLevel, State};

use std::sync::atomic::{AtomicUsize, Ordering};

struct HitCount {
    count: AtomicUsize,
}

const PORT: i32 = 8080;

#[macro_use]
extern crate rocket;

#[get("/")]
fn index(hit_count: &State<HitCount>) -> String {
    let count = hit_count.count.fetch_add(1, Ordering::Relaxed) + 1;
    format!("{:?}", count)
}

async fn get_next_number(client: &Client<HttpConnector>) -> anyhow::Result<i32> {
    let uri: Uri = format!("http://localhost:{:?}", PORT).parse()?;

    let mut resp = client.get(uri).await?;

    let mut resp_bytes: Vec<u8> = vec![];
    while let Some(chunk) = resp.body_mut().data().await {
        resp_bytes.append(&mut chunk?.to_vec());
    }
    let resp_string = String::from_utf8(resp_bytes)?;
    let resp_num = resp_string.parse()?;

    Ok(resp_num)
}

async fn run_client() -> anyhow::Result<()> {
    let client = Client::new();

    get_next_number(&client).await?;

    let start = std::time::Instant::now();
    let requests = 100000;
    for _ in 0..requests {
        get_next_number(&client).await?;
    }
    let elapsed = start.elapsed();
    println!(
        "Performed {:} local requests in {:?} ({:?} / request)",
        requests.to_formatted_string(&Locale::en),
        elapsed,
        elapsed / requests
    );

    Ok(())
}

#[launch]
fn rocket() -> _ {
    std::thread::spawn(|| {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            run_client().await.unwrap();
        });
    });

    let figment = Figment::from(rocket::Config::default())
        .merge(("port", PORT))
        .merge(("log_level", LogLevel::Off));

    rocket::custom(figment)
        .manage(HitCount {
            count: AtomicUsize::new(0),
        })
        .mount("/", routes![index])
}
