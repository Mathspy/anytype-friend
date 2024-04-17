use std::{
    env,
    future::Future,
    io::ErrorKind,
    process::{Command, Stdio},
};

use rand::Rng;

const MACOS_PATH: &str =
    "/Applications/Anytype.app/Contents/Resources/app.asar.unpacked/dist/anytypeHelper";

pub async fn run_with_service<F, Fut, O>(callback: F) -> O
where
    F: FnOnce(u16) -> Fut,
    Fut: Future<Output = O>,
{
    let print_service_output = env::var("ANYTYPE_PRINT_SERVICE_OUTPUT").is_ok();

    let port: u16 = rand::thread_rng().gen_range(9_000..10_000);
    let other_port = port + 1;

    let mut command = Command::new(MACOS_PATH);
    command
        .arg(format!("127.0.0.1:{port}"))
        .arg(format!("127.0.0.1:{other_port}"));

    if !print_service_output {
        command.stdout(Stdio::null()).stderr(Stdio::null());
    }

    let mut child = command.spawn().unwrap();

    loop {
        match tokio::net::TcpStream::connect(format!("127.0.0.1:{port}")).await {
            Ok(_) => break,
            Err(error) if error.kind() == ErrorKind::ConnectionRefused => {
                tokio::task::yield_now().await;
                continue;
            }
            Err(error) => panic!("Failed to connect to TCP server at port {port}:\n{error}"),
        }
    }

    let ret = callback(port).await;

    child.kill().unwrap();

    ret
}
