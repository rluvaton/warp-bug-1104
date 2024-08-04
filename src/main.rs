use anyhow::Result;
use tokio::net::TcpListener;
use tokio_stream::wrappers::TcpListenerStream;
use warp::{Filter, reject, Rejection};
use warp::path::Tail;

fn main() {
    println!("Hello, world!");
}


async fn create_working_tmp_server(listener: TcpListener) -> Result<()> {
    let crates = warp::path("b")
        // Simulating .and(warp::fs::dir(<some-dir>)) for simplicity
        .and(warp::get())
        .and(custom_filter())

        .with(warp::trace::request());

    let problematic_route = warp::path("api")
        .and(warp::put())
        .map(move || {
            "Hello".to_string()
        });


    let routes = problematic_route
        .or(crates);

    warp::serve(routes)
        .run_incoming(TcpListenerStream::new(listener))
        .await;

    Ok(())
}
async fn create_non_working_tmp_server(listener: TcpListener) -> Result<()> {
    let crates = warp::path("b")
        // Simulating .and(warp::fs::dir(<some-dir>)) for simplicity
        .and(warp::get())
        .and(custom_filter())

        .with(warp::trace::request());

    // <-------- This does not work
    let problematic_route =
        warp::put()
            .and(warp::path("api"))
            .map(move || {
                "Hello".to_string()
            });

    let routes = problematic_route
        .or(crates);

    warp::serve(routes)
        .run_incoming(TcpListenerStream::new(listener))
        .await;

    Ok(())
}

fn custom_filter() -> impl Filter<Extract=(String,), Error=Rejection> + Copy {
    warp::path::tail()
        .and_then(|s: Tail| async move {
            let route = s.as_str();

            if !route.ends_with("something") {
                return Err(reject::not_found());
            }

            Ok(route.to_string())
        })
}

#[cfg(test)]
mod tests {
    use std::net::SocketAddr;

    use reqwest::Method;
    use tokio::net::TcpListener;
    use tokio::spawn;
    use tokio::task::JoinHandle;

    use crate::{create_non_working_tmp_server, create_working_tmp_server};

    async fn serve_test(working: bool) -> (JoinHandle<()>, SocketAddr) {
        let port = if working { 5004 } else { 5005 };
        let listener = TcpListener::bind(("127.0.0.1", port)).await.unwrap();
        let addr = listener.local_addr().unwrap();

        let server = move || {
            async move {
                (if working {
                    create_working_tmp_server(listener).await
                } else {
                    create_non_working_tmp_server(listener).await
                }).unwrap()
            }
        };
        let handle = spawn(server());

        (handle, addr)
    }

    #[tokio::test]
    async fn test_not_working() {
        let (_handle, addr) = serve_test(false).await;

        run_test_for_addr(addr).await;
    }

    #[tokio::test]
    async fn test_working() {
        let (_handle, addr) = serve_test(true).await;

        run_test_for_addr(addr).await;
    }

    async fn run_test_for_addr(addr: SocketAddr) {
        let client = reqwest::Client::new();

        let res = client
            .request(Method::GET, format!("http://{}/b/something", addr))
            .send()
            .await
            .expect("Failed to send request")
            .status();

        assert_eq!(res, 200);

        let res = client
            .request(Method::GET, format!("http://{}/b/something-else", addr))
            .send()
            .await
            .expect("Failed to send request")
            .status();

        assert_eq!(res, 404);
    }
}
