use std::{
    env,
    future::Future,
    io::ErrorKind,
    process::{Child, Command, Stdio},
};

use rand::Rng;

const MACOS_PATH: &str =
    "/Applications/Anytype.app/Contents/Resources/app.asar.unpacked/dist/anytypeHelper";

struct KillOnDrop(Child);

impl Drop for KillOnDrop {
    fn drop(&mut self) {
        let _ = self.0.kill();
    }
}

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

    let _child = KillOnDrop(command.spawn().unwrap());

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

    callback(port).await
}

#[macro_export]
macro_rules! assert_relations_eq {
    ($a:expr, $b:expr) => {
        let equal = match ($a, $b) {
            (RelationValue::Text(a), RelationValue::Text(b))
            | (RelationValue::Url(a), RelationValue::Url(b))
            | (RelationValue::Email(a), RelationValue::Email(b))
            | (RelationValue::Phone(a), RelationValue::Phone(b)) => a == b,
            (RelationValue::Number(a), RelationValue::Number(b)) => a == b,
            (RelationValue::Date(a), RelationValue::Date(b)) => a == b,
            (RelationValue::Checkbox(a), RelationValue::Checkbox(b)) => a == b,
            (RelationValue::Object(a), RelationValue::Object(b)) => {
                use ::std::collections::HashSet;
                a.into_iter()
                    .map(|object| object.id().clone())
                    .collect::<HashSet<_>>()
                    == b.into_iter()
                        .map(|object| object.id().clone())
                        .collect::<HashSet<_>>()
            }
            _ => false,
        };

        if !equal {
            panic!(
                "assertion `left == right` failed
left: {:?}
right: {:?}",
                $a, $b
            )
        }
    };
}
