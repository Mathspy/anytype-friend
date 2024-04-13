mod pb {
    mod models {
        tonic::include_proto!("anytype.model");
    }

    tonic::include_proto!("anytype");
}

fn main() {
    println!("Hello, world!");
}
